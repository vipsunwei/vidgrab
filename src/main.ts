import { createApp } from "vue";
import 'uno.css'
import './style.css'
import App from "./App.vue";

// 固定暗黑风格
document.documentElement.classList.add('dark')

createApp(App).mount("#app");
