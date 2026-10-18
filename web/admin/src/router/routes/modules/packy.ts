import { DEFAULT_LAYOUT } from '../base';
export default {path:'/packy', name:'Packy', component:DEFAULT_LAYOUT, meta:{locale:'Packy',requiresAuth:true,icon:'icon-apps',order:0}, children:[
 {path:'packs',name:'Packs',component:()=>import('@/views/packy/packs.vue'),meta:{locale:'整合包与发布',requiresAuth:true}},
 {path:'mods',name:'Mods',component:()=>import('@/views/packy/mods.vue'),meta:{locale:'Mod 与文件',requiresAuth:true}},
 {path:'modrinth',name:'Modrinth',component:()=>import('@/views/packy/modrinth.vue'),meta:{locale:'Modrinth 导入',requiresAuth:true}},
 {path:'sources',name:'Sources',component:()=>import('@/views/packy/sources.vue'),meta:{locale:'下载来源',requiresAuth:true}},
 {path:'audit',name:'Audit',component:()=>import('@/views/packy/audit.vue'),meta:{locale:'审计日志',requiresAuth:true}}
]};
