import pluginVue from "eslint-plugin-vue";
import { defineConfigWithVueTs, vueTsConfigs } from "@vue/eslint-config-typescript";

export default defineConfigWithVueTs(
  { ignores: ["dist/**", "src-tauri/**", "node_modules/**", "*.config.*"] },
  pluginVue.configs["flat/recommended"],
  vueTsConfigs.strictTypeChecked,
  {
    rules: {
      "vue/multi-word-component-names": "off",
      // Prettier biçimlendirir; çakışan stil kuralları kapalı.
      "vue/max-attributes-per-line": "off",
      "vue/singleline-html-element-content-newline": "off",
      "vue/html-self-closing": "off",
      "vue/html-indent": "off",
      "vue/html-closing-bracket-newline": "off",
      "vue/multiline-html-element-content-newline": "off",
    },
  },
);
