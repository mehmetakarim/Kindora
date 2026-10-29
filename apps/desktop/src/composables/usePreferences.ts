import { ref, watch, type Ref } from "vue";

/**
 * Filiz tercihleri (P1.C): sessiz mod ve hareketi azalt. Yalnız bu bilgisayarda,
 * WebView yerel deposunda tutulur; agent kurallarını etkilemez. Ses bu sürümde
 * yoktur (varsayılan kapalı). Depo kullanılamazsa varsayılanlarla çalışır.
 */
export interface Preferences {
  quiet: Ref<boolean>;
  reduceMotion: Ref<boolean>;
}

const KEY = "kd.prefs.v1";

function load(): { quiet: boolean; reduceMotion: boolean } {
  try {
    const raw = window.localStorage.getItem(KEY);
    if (raw !== null) {
      const v = JSON.parse(raw) as Partial<{ quiet: unknown; reduceMotion: unknown }>;
      return { quiet: v.quiet === true, reduceMotion: v.reduceMotion === true };
    }
  } catch {
    // Depo yok veya bozuk: varsayılanlar.
  }
  return { quiet: false, reduceMotion: false };
}

export function usePreferences(): Preferences {
  const initial = load();
  const quiet = ref(initial.quiet);
  const reduceMotion = ref(initial.reduceMotion);
  watch([quiet, reduceMotion], () => {
    try {
      window.localStorage.setItem(
        KEY,
        JSON.stringify({ quiet: quiet.value, reduceMotion: reduceMotion.value }),
      );
    } catch {
      // Kaydedilemedi: tercih yalnız bu oturumda geçerli.
    }
  });
  return { quiet, reduceMotion };
}
