(self.webpackChunk_N_E=self.webpackChunk_N_E||[]).push([[127],{433:function(n,e,i){(window.__NEXT_P=window.__NEXT_P||[]).push(["/sources",function(){return i(8999)}])},30287:function(n,e,i){"use strict";i.d(e,{KB:function(){return Relations},Kf:function(){return k},gU:function(){return T},vk:function(){return v},vP:function(){return w},sW:function(){return C}});var t=i(85893),r=i(47741),o=i(39653),l=i(50471),c=i(40639),a=i(36696),s=i(63679),d=i(9008),u=i.n(d),h=i(41664),m=i.n(h),x=i(67294),j=i(44527),f=i(10068);function extractColumnInfo(n){var e,i,t,r;return"columnDesc"in n?"".concat(null===(e=n.columnDesc)||void 0===e?void 0:e.name," (").concat(null===(t=n.columnDesc)||void 0===t?void 0:null===(i=t.columnType)||void 0===i?void 0:i.typeName,")"):"".concat(n.name," (").concat(null===(r=n.dataType)||void 0===r?void 0:r.typeName,")")}let p=(0,s.ZP)(()=>i.e(171).then(i.t.bind(i,55171,23))),v={name:"Depends",width:1,content:n=>(0,t.jsx)(m(),{href:"/streaming_graph/?id=".concat(n.id),children:(0,t.jsx)(r.zx,{size:"sm","aria-label":"view dependents",colorScheme:"blue",variant:"link",children:"D"})})},w={name:"Primary Key",width:1,content:n=>n.pk.map(n=>n.columnIndex).map(e=>n.columns[e]).map(n=>extractColumnInfo(n)).join(", ")},T={name:"Connector",width:3,content:n=>{var e;return null!==(e=n.withProperties.connector)&&void 0!==e?e:"unknown"}},k={name:"Connector",width:3,content:n=>{var e;return null!==(e=n.properties.connector)&&void 0!==e?e:"unknown"}},C=[v,{name:"Fragments",width:1,content:n=>(0,t.jsx)(m(),{href:"/streaming_plan/?id=".concat(n.id),children:(0,t.jsx)(r.zx,{size:"sm","aria-label":"view fragments",colorScheme:"blue",variant:"link",children:"F"})})}];function Relations(n,e,i){let s=(0,f.Z)(),[d,h]=(0,x.useState)([]);(0,x.useEffect)(()=>((async function(){try{h(await e())}catch(n){s(n)}})(),()=>{}),[s,e]);let{isOpen:m,onOpen:v,onClose:w}=(0,o.qY)(),[T,k]=(0,x.useState)(),openRelationCatalog=n=>{n&&(k(n),v())},C=(0,t.jsxs)(l.u_,{isOpen:m,onClose:w,size:"3xl",children:[(0,t.jsx)(l.ZA,{}),(0,t.jsxs)(l.hz,{children:[(0,t.jsxs)(l.xB,{children:["Catalog of ",null==T?void 0:T.id," - ",null==T?void 0:T.name]}),(0,t.jsx)(l.ol,{}),(0,t.jsx)(l.fe,{children:m&&T&&(0,t.jsx)(p,{src:T,collapsed:1,name:null,displayDataTypes:!1})}),(0,t.jsx)(l.mz,{children:(0,t.jsx)(r.zx,{colorScheme:"blue",mr:3,onClick:w,children:"Close"})})]})]}),_=(0,t.jsxs)(c.xu,{p:3,children:[(0,t.jsx)(j.Z,{children:n}),(0,t.jsx)(a.xJ,{children:(0,t.jsxs)(a.iA,{variant:"simple",size:"sm",maxWidth:"full",children:[(0,t.jsx)(a.hr,{children:(0,t.jsxs)(a.Tr,{children:[(0,t.jsx)(a.Th,{width:3,children:"Id"}),(0,t.jsx)(a.Th,{width:5,children:"Name"}),(0,t.jsx)(a.Th,{width:3,children:"Owner"}),i.map(n=>(0,t.jsx)(a.Th,{width:n.width,children:n.name},n.name)),(0,t.jsx)(a.Th,{children:"Visible Columns"})]})}),(0,t.jsx)(a.p3,{children:d.map(n=>(0,t.jsxs)(a.Tr,{children:[(0,t.jsx)(a.Td,{children:(0,t.jsx)(r.zx,{size:"sm","aria-label":"view catalog",colorScheme:"blue",variant:"link",onClick:()=>openRelationCatalog(n),children:n.id})}),(0,t.jsx)(a.Td,{children:n.name}),(0,t.jsx)(a.Td,{children:n.owner}),i.map(e=>(0,t.jsx)(a.Td,{children:e.content(n)},e.name)),(0,t.jsx)(a.Td,{overflowWrap:"normal",children:n.columns.filter(n=>!("isHidden"in n)||!n.isHidden).map(n=>extractColumnInfo(n)).join(", ")})]},n.id))})]})})]});return(0,t.jsxs)(x.Fragment,{children:[(0,t.jsx)(u(),{children:(0,t.jsx)("title",{children:n})}),C,_]})}},8999:function(n,e,i){"use strict";i.r(e),i.d(e,{default:function(){return DataSources}});var t=i(30287),r=i(35413);function DataSources(){return(0,t.KB)("Sources",r.e6,[t.gU,{name:"Row Format",width:3,content:n=>{var e,i;return null!==(i=null===(e=n.info)||void 0===e?void 0:e.rowFormat)&&void 0!==i?i:"unknown"}},t.vk])}}},function(n){n.O(0,[662,184,476,155,284,413,774,888,179],function(){return n(n.s=433)}),_N_E=n.O()}]);