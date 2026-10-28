import "@fontsource/onest/400.css";
import "@fontsource/onest/500.css";
import "@fontsource/onest/600.css";
import "@fontsource/onest/700.css";
import "@fontsource/fredoka/500.css";
import "@fontsource/fredoka/600.css";
import "./styles/main.scss";

import { createApp } from "vue";
import App from "./App.vue";
import { applyEnvironmentPreferences } from "./theme";

const motion = applyEnvironmentPreferences(document.documentElement, window);
createApp(App, { motion }).mount("#app");
