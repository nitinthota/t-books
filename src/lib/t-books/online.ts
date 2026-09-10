export function isBrowserOnline(): boolean {
  if (typeof navigator === "undefined") return true;
  return navigator.onLine !== false;
}

export function subscribeOnline(listener: (online: boolean) => void): () => void {
  if (typeof window === "undefined") return () => {};
  const on = () => listener(true);
  const off = () => listener(false);
  window.addEventListener("online", on);
  window.addEventListener("offline", off);
  return () => {
    window.removeEventListener("online", on);
    window.removeEventListener("offline", off);
  };
}
