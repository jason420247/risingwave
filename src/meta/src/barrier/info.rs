// Copyright 2025 RisingWave Labs
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use itertools::Itertools;
use risingwave_common::bitmap::Bitmap;
use risingwave_common::catalog::{DatabaseId, TableId};
use risingwave_common::util::stream_graph_visitor::visit_stream_node_mut;
use risingwave_meta_model::WorkerId;
use risingwave_pb::meta::subscribe_response::Operation;
use risingwave_pb::stream_plan::PbSubscriptionUpstreamInfo;
use risingwave_pb::stream_plan::stream_node::NodeBody;
use tracing::warn;

use crate::barrier::edge_builder::{FragmentEdgeBuildResult, FragmentEdgeBuilder};
use crate::barrier::rpc::ControlStreamManager;
use crate::barrier::{BarrierKind, Command, CreateStreamingJobType, TracedEpoch};
use crate::controller::fragment::{InflightActorInfo, InflightFragmentInfo};
use crate::controller::utils::rebuild_fragment_mapping;
use crate::manager::NotificationManagerRef;
use crate::model::{ActorId, FragmentId, SubscriptionId};

#[derive(Clone, educe::Educe)]
#[educe(Debug)]
pub(crate) struct SharedActorInfos {
    inner: Arc<parking_lot::RwLock<HashMap<DatabaseId, HashMap<FragmentId, InflightFragmentInfo>>>>,
    #[educe(Debug(ignore))]
    notification_manager: NotificationManagerRef,
}

impl SharedActorInfos {
    pub(crate) fn new(notification_manager: NotificationManagerRef) -> Self {
        Self {
            inner: Arc::new(Default::default()),
            notification_manager,
        }
    }

    pub(super) fn remove_database(&self, database_id: DatabaseId) {
        if let Some(database) = self.inner.write().remove(&database_id) {
            let mapping = database
                .into_values()
                .map(|fragment| rebuild_fragment_mapping(&fragment))
                .collect_vec();
            if !mapping.is_empty() {
                self.notification_manager
                    .notify_fragment_mapping(Operation::Delete, mapping);
            }
        }
    }

    pub(super) fn retain_databases(&self, database_ids: impl IntoIterator<Item = DatabaseId>) {
        let database_ids: HashSet<_> = database_ids.into_iter().collect();

        let mut mapping = Vec::new();
        for fragment in self
            .inner
            .write()
            .extract_if(|database_id, _| !database_ids.contains(database_id))
            .flat_map(|(_, fragments)| fragments.into_values())
        {
            mapping.push(rebuild_fragment_mapping(&fragment));
        }
        if !mapping.is_empty() {
            self.notification_manager
                .notify_fragment_mapping(Operation::Delete, mapping);
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct BarrierInfo {
    pub prev_epoch: TracedEpoch,
    pub curr_epoch: TracedEpoch,
    pub kind: BarrierKind,
}

impl BarrierInfo {
    pub(super) fn prev_epoch(&self) -> u64 {
        self.prev_epoch.value().0
    }
}

#[derive(Debug, Clone)]
pub(crate) enum CommandFragmentChanges {
    NewFragment(TableId, InflightFragmentInfo),
    ReplaceNodeUpstream(
        /// old `fragment_id` -> new `fragment_id`
        HashMap<FragmentId, FragmentId>,
    ),
    Reschedule {
        new_actors: HashMap<ActorId, InflightActorInfo>,
        actor_update_vnode_bitmap: HashMap<ActorId, Bitmap>,
        to_remove: HashSet<ActorId>,
    },
    RemoveFragment,
}

#[derive(Default, Clone, Debug)]
pub struct InflightSubscriptionInfo {
    /// `mv_table_id` => `subscription_id` => retention seconds
    pub mv_depended_subscriptions: HashMap<TableId, HashMap<SubscriptionId, u64>>,
}

#[derive(Clone, Debug)]
pub struct InflightStreamingJobInfo {
    pub job_id: TableId,
    pub fragment_infos: HashMap<FragmentId, InflightFragmentInfo>,
}

impl InflightStreamingJobInfo {
    pub fn fragment_infos(&self) -> impl Iterator<Item = &InflightFragmentInfo> + '_ {
        self.fragment_infos.values()
    }

    pub fn existing_table_ids(&self) -> impl Iterator<Item = TableId> + '_ {
        InflightFragmentInfo::existing_table_ids(self.fragment_infos())
    }
}

impl<'a> IntoIterator for &'a InflightStreamingJobInfo {
    type Item = &'a InflightFragmentInfo;

    type IntoIter = impl Iterator<Item = &'a InflightFragmentInfo> + 'a;

    fn into_iter(self) -> Self::IntoIter {
        self.fragment_infos()
    }
}

#[derive(Clone, Debug)]
pub struct InflightDatabaseInfo {
    database_id: DatabaseId,
    jobs: HashMap<TableId, InflightStreamingJobInfo>,
    fragment_location: HashMap<FragmentId, TableId>,
    shared_actor_infos: SharedActorInfos,
}

impl InflightDatabaseInfo {
    pub fn fragment_infos(&self) -> impl Iterator<Item = &InflightFragmentInfo> + '_ {
        self.jobs.values().flat_map(|job| job.fragment_infos())
    }

    pub fn contains_job(&self, job_id: TableId) -> bool {
        self.jobs.contains_key(&job_id)
    }

    pub fn fragment(&self, fragment_id: FragmentId) -> &InflightFragmentInfo {
        let job_id = self.fragment_location[&fragment_id];
        self.jobs
            .get(&job_id)
            .expect("should exist")
            .fragment_infos
            .get(&fragment_id)
            .expect("should exist")
    }

    fn fragment_mut(&mut self, fragment_id: FragmentId) -> &mut InflightFragmentInfo {
        let job_id = self.fragment_location[&fragment_id];
        self.jobs
            .get_mut(&job_id)
            .expect("should exist")
            .fragment_infos
            .get_mut(&fragment_id)
            .expect("should exist")
    }

    fn empty_inner(database_id: DatabaseId, shared_actor_infos: SharedActorInfos) -> Self {
        Self {
            database_id,
            jobs: Default::default(),
            fragment_location: Default::default(),
            shared_actor_infos,
        }
    }

    pub fn empty(database_id: DatabaseId, shared_actor_infos: SharedActorInfos) -> Self {
        // remove the database because it's empty.
        shared_actor_infos.remove_database(database_id);
        Self::empty_inner(database_id, shared_actor_infos)
    }

    pub fn recover(
        database_id: DatabaseId,
        jobs: impl Iterator<Item = InflightStreamingJobInfo>,
        shared_actor_infos: SharedActorInfos,
    ) -> Self {
        let mut info = Self::empty_inner(database_id, shared_actor_infos);
        for job in jobs {
            info.extend_inner(job);
        }

        // delete the fragments that exist previously, but not included in the recovered fragments
        let mut guard = info.shared_actor_infos.inner.write();
        let database = guard.entry(database_id).or_default();
        let mut deleted_mapping = Vec::new();
        for (_, fragment) in
            database.extract_if(|fragment_id, _| !info.fragment_location.contains_key(fragment_id))
        {
            deleted_mapping.push(rebuild_fragment_mapping(&fragment));
        }
        if !deleted_mapping.is_empty() {
            info.shared_actor_infos
                .notification_manager
                .notify_fragment_mapping(Operation::Delete, deleted_mapping);
        }

        drop(guard);
        info
    }

    pub fn extend(&mut self, job: InflightStreamingJobInfo) {
        self.extend_inner(job)
    }

    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    fn extend_inner(&mut self, job: InflightStreamingJobInfo) {
        self.apply_add(job.fragment_infos.into_iter().map(|(fragment_id, info)| {
            (
                fragment_id,
                CommandFragmentChanges::NewFragment(job.job_id, info),
            )
        }))
    }

    /// Apply some actor changes before issuing a barrier command, if the command contains any new added actors, we should update
    /// the info correspondingly.
    pub(crate) fn pre_apply(
        &mut self,
        fragment_changes: &HashMap<FragmentId, CommandFragmentChanges>,
    ) {
        self.apply_add(
            fragment_changes
                .iter()
                .map(|(fragment_id, change)| (*fragment_id, change.clone())),
        )
    }

    fn apply_add(
        &mut self,
        fragment_changes: impl Iterator<Item = (FragmentId, CommandFragmentChanges)>,
    ) {
        let mut added_mapping = Vec::new();
        let mut updated_mapping = Vec::new();
        let inner = self.shared_actor_infos.inner.clone();
        let mut guard = inner.write();
        let database = guard.entry(self.database_id).or_default();
        {
            for (fragment_id, change) in fragment_changes {
                match change {
                    CommandFragmentChanges::NewFragment(job_id, info) => {
                        let fragment_infos =
                            self.jobs
                                .entry(job_id)
                                .or_insert_with(|| InflightStreamingJobInfo {
                                    job_id,
                                    fragment_infos: Default::default(),
                                });
                        match database.entry(fragment_id) {
                            Entry::Occupied(mut entry) => {
                                entry.insert(info.clone());
                                updated_mapping.push(rebuild_fragment_mapping(&info));
                            }
                            Entry::Vacant(entry) => {
                                entry.insert(info.clone());
                                added_mapping.push(rebuild_fragment_mapping(&info));
                            }
                        }
                        fragment_infos
                            .fragment_infos
                            .try_insert(fragment_id, info)
                            .expect("non duplicate");
                        self.fragment_location
                            .try_insert(fragment_id, job_id)
                            .expect("non duplicate");
                    }
                    CommandFragmentChanges::Reschedule {
                        new_actors,
                        actor_update_vnode_bitmap,
                        ..
                    } => {
                        let info = self.fragment_mut(fragment_id);
                        let actors = &mut info.actors;
                        for (actor_id, new_vnodes) in actor_update_vnode_bitmap {
                            actors
                                .get_mut(&actor_id)
                                .expect("should exist")
                                .vnode_bitmap = Some(new_vnodes);
                        }
                        for (actor_id, actor) in new_actors {
                            actors
                                .try_insert(actor_id as _, actor)
                                .expect("non-duplicate");
                        }
                    }
                    CommandFragmentChanges::RemoveFragment => {}
                    CommandFragmentChanges::ReplaceNodeUpstream(replace_map) => {
                        let mut remaining_fragment_ids: HashSet<_> =
                            replace_map.keys().cloned().collect();
                        let info = self.fragment_mut(fragment_id);
                        visit_stream_node_mut(&mut info.nodes, |node| {
                            if let NodeBody::Merge(m) = node
                                && let Some(new_upstream_fragment_id) =
                                    replace_map.get(&m.upstream_fragment_id)
                            {
                                if !remaining_fragment_ids.remove(&m.upstream_fragment_id) {
                                    if cfg!(debug_assertions) {
                                        panic!(
                                            "duplicate upstream fragment: {:?} {:?}",
                                            m, replace_map
                                        );
                                    } else {
                                        warn!(?m, ?replace_map, "duplicate upstream fragment");
                                    }
                                }
                                m.upstream_fragment_id = *new_upstream_fragment_id;
                            }
                        });
                        if cfg!(debug_assertions) {
                            assert!(
                                remaining_fragment_ids.is_empty(),
                                "non-existing fragment to replace: {:?} {:?} {:?}",
                                remaining_fragment_ids,
                                info.nodes,
                                replace_map
                            );
                        } else {
                            warn!(?remaining_fragment_ids, node = ?info.nodes, ?replace_map, "non-existing fragment to replace");
                        }
                    }
                }
            }
        }
        if !added_mapping.is_empty() {
            self.shared_actor_infos
                .notification_manager
                .notify_fragment_mapping(Operation::Add, added_mapping);
        }
        if !updated_mapping.is_empty() {
            self.shared_actor_infos
                .notification_manager
                .notify_fragment_mapping(Operation::Update, updated_mapping);
        }
    }

    pub(super) fn build_edge(
        &self,
        command: Option<&Command>,
        control_stream_manager: &ControlStreamManager,
    ) -> Option<FragmentEdgeBuildResult> {
        let (info, replace_job) = match command {
            None => {
                return None;
            }
            Some(command) => match command {
                Command::Flush
                | Command::Pause
                | Command::Resume
                | Command::DropStreamingJobs { .. }
                | Command::MergeSnapshotBackfillStreamingJobs(_)
                | Command::RescheduleFragment { .. }
                | Command::SourceChangeSplit(_)
                | Command::Throttle(_)
                | Command::CreateSubscription { .. }
                | Command::DropSubscription { .. }
                | Command::ConnectorPropsChange(_)
                | Command::StartFragmentBackfill { .. } => {
                    return None;
                }
                Command::CreateStreamingJob { info, job_type, .. } => {
                    let replace_job = match job_type {
                        CreateStreamingJobType::Normal
                        | CreateStreamingJobType::SnapshotBackfill(_) => None,
                        CreateStreamingJobType::SinkIntoTable(replace_job) => Some(replace_job),
                    };
                    (Some(info), replace_job)
                }
                Command::ReplaceStreamJob(replace_job) => (None, Some(replace_job)),
            },
        };
        // `existing_fragment_ids` consists of
        //  - keys of `info.upstream_fragment_downstreams`, which are the `fragment_id` the upstream fragment of the newly created job
        //  - keys of `replace_job.upstream_fragment_downstreams`, which are the `fragment_id` of upstream fragment of replace_job,
        // if the upstream fragment previously exists
        //  - keys of `replace_upstream`, which are the `fragment_id` of downstream fragments that will update their upstream fragments.
        let existing_fragment_ids = info
            .into_iter()
            .flat_map(|info| info.upstream_fragment_downstreams.keys())
            .chain(replace_job.into_iter().flat_map(|replace_job| {
                replace_job
                    .upstream_fragment_downstreams
                    .keys()
                    .filter(|fragment_id| {
                        info.map(|info| {
                            !info
                                .stream_job_fragments
                                .fragments
                                .contains_key(fragment_id)
                        })
                        .unwrap_or(true)
                    })
                    .chain(replace_job.replace_upstream.keys())
            }))
            .cloned();
        let new_fragment_infos = info
            .into_iter()
            .flat_map(|info| info.stream_job_fragments.new_fragment_info())
            .chain(replace_job.into_iter().flat_map(|replace_job| {
                replace_job.new_fragments.new_fragment_info().chain(
                    replace_job
                        .auto_refresh_schema_sinks
                        .as_ref()
                        .into_iter()
                        .flat_map(|sinks| {
                            sinks.iter().map(|sink| {
                                (sink.new_fragment.fragment_id, sink.new_fragment_info())
                            })
                        }),
                )
            }))
            .collect_vec();
        let mut builder = FragmentEdgeBuilder::new(
            existing_fragment_ids
                .map(|fragment_id| self.fragment(fragment_id))
                .chain(new_fragment_infos.iter().map(|(_, info)| info)),
            control_stream_manager,
        );
        if let Some(info) = info {
            builder.add_relations(&info.upstream_fragment_downstreams);
            builder.add_relations(&info.stream_job_fragments.downstreams);
        }
        if let Some(replace_job) = replace_job {
            builder.add_relations(&replace_job.upstream_fragment_downstreams);
            builder.add_relations(&replace_job.new_fragments.downstreams);
        }
        if let Some(replace_job) = replace_job {
            for (fragment_id, fragment_replacement) in &replace_job.replace_upstream {
                for (original_upstream_fragment_id, new_upstream_fragment_id) in
                    fragment_replacement
                {
                    builder.replace_upstream(
                        *fragment_id,
                        *original_upstream_fragment_id,
                        *new_upstream_fragment_id,
                    );
                }
            }
        }
        Some(builder.build())
    }
}

impl InflightSubscriptionInfo {
    pub fn pre_apply(&mut self, command: &Command) {
        if let Command::CreateSubscription {
            subscription_id,
            upstream_mv_table_id,
            retention_second,
        } = command
            && let Some(prev_retiontion) = self
                .mv_depended_subscriptions
                .entry(*upstream_mv_table_id)
                .or_default()
                .insert(*subscription_id, *retention_second)
        {
            warn!(subscription_id, ?upstream_mv_table_id, mv_depended_subscriptions = ?self.mv_depended_subscriptions, prev_retiontion, "add an existing subscription id");
        }
    }
}

impl<'a> IntoIterator for &'a InflightSubscriptionInfo {
    type Item = PbSubscriptionUpstreamInfo;

    type IntoIter = impl Iterator<Item = PbSubscriptionUpstreamInfo> + 'a;

    fn into_iter(self) -> Self::IntoIter {
        self.mv_depended_subscriptions
            .iter()
            .flat_map(|(table_id, subscriptions)| {
                subscriptions
                    .keys()
                    .map(|subscriber_id| PbSubscriptionUpstreamInfo {
                        subscriber_id: *subscriber_id,
                        upstream_mv_table_id: table_id.table_id,
                    })
            })
    }
}

impl InflightDatabaseInfo {
    /// Apply some actor changes after the barrier command is collected, if the command contains any actors that are dropped, we should
    /// remove that from the snapshot correspondingly.
    pub(crate) fn post_apply(
        &mut self,
        fragment_changes: &HashMap<FragmentId, CommandFragmentChanges>,
    ) {
        let mut updated_mapping = Vec::new();
        let mut deleted_mapping = Vec::new();
        let inner = self.shared_actor_infos.inner.clone();
        let mut guard = inner.write();
        let database = guard.entry(self.database_id).or_default();
        {
            for (fragment_id, changes) in fragment_changes {
                match changes {
                    CommandFragmentChanges::NewFragment(_, _) => {}
                    CommandFragmentChanges::Reschedule { to_remove, .. } => {
                        let job_id = self.fragment_location[fragment_id];
                        let info = self
                            .jobs
                            .get_mut(&job_id)
                            .expect("should exist")
                            .fragment_infos
                            .get_mut(fragment_id)
                            .expect("should exist");
                        for actor_id in to_remove {
                            assert!(info.actors.remove(&(*actor_id as _)).is_some());
                        }
                        database.insert(*fragment_id, info.clone());
                        updated_mapping.push(rebuild_fragment_mapping(info));
                    }
                    CommandFragmentChanges::RemoveFragment => {
                        let job_id = self
                            .fragment_location
                            .remove(fragment_id)
                            .expect("should exist");
                        let job = self.jobs.get_mut(&job_id).expect("should exist");
                        let fragment = job
                            .fragment_infos
                            .remove(fragment_id)
                            .expect("should exist");
                        database.remove(fragment_id);
                        deleted_mapping.push(rebuild_fragment_mapping(&fragment));
                        if job.fragment_infos.is_empty() {
                            self.jobs.remove(&job_id).expect("should exist");
                        }
                    }
                    CommandFragmentChanges::ReplaceNodeUpstream(_) => {}
                }
            }
        }
        if !updated_mapping.is_empty() {
            self.shared_actor_infos
                .notification_manager
                .notify_fragment_mapping(Operation::Update, updated_mapping);
        }
        if !deleted_mapping.is_empty() {
            self.shared_actor_infos
                .notification_manager
                .notify_fragment_mapping(Operation::Delete, deleted_mapping);
        }
    }
}

impl InflightSubscriptionInfo {
    pub fn post_apply(&mut self, command: &Command) {
        if let Command::DropSubscription {
            subscription_id,
            upstream_mv_table_id,
        } = command
        {
            let removed = match self.mv_depended_subscriptions.get_mut(upstream_mv_table_id) {
                Some(subscriptions) => {
                    let removed = subscriptions.remove(subscription_id).is_some();
                    if removed && subscriptions.is_empty() {
                        self.mv_depended_subscriptions.remove(upstream_mv_table_id);
                    }
                    removed
                }
                None => false,
            };
            if !removed {
                warn!(subscription_id, ?upstream_mv_table_id, mv_depended_subscriptions = ?self.mv_depended_subscriptions, "remove a non-existing subscription id");
            }
        }
    }
}

impl InflightFragmentInfo {
    /// Returns actor list to collect in the target worker node.
    pub(crate) fn actor_ids_to_collect(
        infos: impl IntoIterator<Item = &Self>,
    ) -> HashMap<WorkerId, HashSet<ActorId>> {
        let mut ret: HashMap<_, HashSet<_>> = HashMap::new();
        for (actor_id, actor) in infos.into_iter().flat_map(|info| info.actors.iter()) {
            assert!(
                ret.entry(actor.worker_id)
                    .or_default()
                    .insert(*actor_id as _)
            )
        }
        ret
    }

    pub fn existing_table_ids<'a>(
        infos: impl IntoIterator<Item = &'a Self> + 'a,
    ) -> impl Iterator<Item = TableId> + 'a {
        infos
            .into_iter()
            .flat_map(|info| info.state_table_ids.iter().cloned())
    }

    pub fn contains_worker(infos: impl IntoIterator<Item = &Self>, worker_id: WorkerId) -> bool {
        infos.into_iter().any(|fragment| {
            fragment
                .actors
                .values()
                .any(|actor| (actor.worker_id) == worker_id)
        })
    }

    pub(crate) fn workers(infos: impl IntoIterator<Item = &Self>) -> HashSet<WorkerId> {
        infos
            .into_iter()
            .flat_map(|info| info.actors.values())
            .map(|actor| actor.worker_id)
            .collect()
    }
}

impl InflightDatabaseInfo {
    pub fn contains_worker(&self, worker_id: WorkerId) -> bool {
        InflightFragmentInfo::contains_worker(self.fragment_infos(), worker_id)
    }

    pub fn existing_table_ids(&self) -> impl Iterator<Item = TableId> + '_ {
        InflightFragmentInfo::existing_table_ids(self.fragment_infos())
    }
}
