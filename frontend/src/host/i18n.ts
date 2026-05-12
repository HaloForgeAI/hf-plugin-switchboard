export function useI18nStore<T>(selector: (state: { locale: "en" | "zh" }) => T): T {
  const stored = window.localStorage.getItem("hf:locale");
  const locale = stored === "zh" || stored === "en"
    ? stored
    : navigator.language.toLowerCase().startsWith("zh") ? "zh" : "en";
  return selector({ locale });
}
