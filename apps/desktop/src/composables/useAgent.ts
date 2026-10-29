import { computed, onBeforeUnmount, onMounted, ref, shallowRef } from "vue";
import { connectionStatus } from "../agent/presentation";
import type { AgentSource, AgentView } from "../agent/types";

/** Agent durumunu kaynaktan okur, olaylara abone olur ve "şimdi"yi saniyede bir günceller. */
export function useAgent(source: AgentSource) {
  const view = shallowRef<AgentView | null>(null);
  const now = ref(Date.now());
  const refreshing = ref(false);
  const error = ref<string | null>(null);
  let unsubscribe: (() => void) | null = null;
  let timer: ReturnType<typeof setInterval> | null = null;

  onMounted(async () => {
    timer = setInterval(() => {
      now.value = Date.now();
    }, 1000);
    try {
      unsubscribe = await source.subscribe((v) => {
        view.value = v;
      });
      view.value ??= await source.initial();
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    }
  });

  onBeforeUnmount(() => {
    if (timer !== null) clearInterval(timer);
    unsubscribe?.();
  });

  async function refresh() {
    if (refreshing.value) return;
    refreshing.value = true;
    error.value = null;
    try {
      await source.refresh();
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    } finally {
      // Rust tarafı da 1 sn'de birden sık iletmez; düğme kısa süre kilitli kalır.
      setTimeout(() => {
        refreshing.value = false;
      }, 1000);
    }
  }

  const status = computed(() => (view.value ? connectionStatus(view.value) : null));

  return { view, now, refreshing, error, status, refresh, kind: source.kind };
}
