// Tema ve hareket tercihleri. Tasarım: açık tema varsayılan ("Görünüm ve dil
// (Light varsayılan)"); koyu token'lar `data-bs-theme="dark"` ile hazır. Uygulama
// içi tema seçimi eklendiğinde yalnız `applyTheme` çağrılır.

export type Theme = "light" | "dark";

export const DEFAULT_THEME: Theme = "light";

export function applyTheme(root: HTMLElement, theme: Theme): void {
  root.setAttribute("data-bs-theme", theme);
}

/** Temayı ve azaltılmış hareketi uygular; maskot hareketi açıksa `true` döner. */
export function applyEnvironmentPreferences(
  root: HTMLElement,
  win: Pick<Window, "matchMedia">,
): boolean {
  applyTheme(root, DEFAULT_THEME);
  const reduced = win.matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (reduced) root.setAttribute("data-kd-motion", "reduced");
  return !reduced;
}
