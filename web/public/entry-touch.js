// The reverse guard: a fine pointer (mouse/trackpad) landing on the touch entry — a bookmark,
// a shared link — is sent back to the desktop entry before first paint. `?ui=` is the same
// undocumented, dev-only override as entry-desktop.js (spec §8.7).
(function () {
  var params = new URLSearchParams(location.search);
  var override = params.get("ui");
  if (override === "touch") return;
  var coarse = window.matchMedia("(pointer: coarse)").matches;
  if (override === "desktop" || !coarse) {
    location.replace("/" + location.search + location.hash);
  }
})();
