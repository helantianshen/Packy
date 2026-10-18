import {App} from 'vue';
import Breadcrumb from './breadcrumb/index.vue';
export default {install(app:App){app.component('Breadcrumb',Breadcrumb);}};
