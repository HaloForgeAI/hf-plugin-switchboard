import { defineModulePlugin, registerPlugin } from "@haloforge/plugin-sdk";
import { SwitchboardPanel } from "./SwitchboardPanel";
import "./styles.css";

const PLUGIN_ID = "dev.haloforge.switchboard";

registerPlugin(
  PLUGIN_ID,
  defineModulePlugin({
    component: SwitchboardPanel,
  }),
);
