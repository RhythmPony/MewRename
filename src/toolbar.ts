import { getCurrentWindow } from '@tauri-apps/api/window';


window.addEventListener("DOMContentLoaded", () => {
  const appWindow = getCurrentWindow();

  document.getElementById('titlebar-minimize')
    ?.addEventListener('click', () => appWindow.minimize());
  document.getElementById('titlebar-maximize')
    ?.addEventListener('click', () => appWindow.toggleMaximize());
  document.getElementById('titlebar-close')
    ?.addEventListener('click', () => appWindow.close());
});