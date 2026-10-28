import { isTauri } from "@tauri-apps/api/core";
import { createDemoSource } from "./demoSource";
import { createTauriSource } from "./tauriSource";
import type { AgentSource } from "./types";

/**
 * Tauri içinde her zaman gerçek kaynak. Demo yalnız Tauri dışında (tarayıcı)
 * ve ekranda "Demo — gerçek Agent verisi değil" bandıyla kullanılır.
 */
export function createAgentSource(inTauri: boolean = isTauri()): AgentSource {
  return inTauri ? createTauriSource() : createDemoSource();
}
