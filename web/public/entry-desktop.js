// Router: a coarse pointer (touch) is sent to the dedicated touch entry before first paint,
// so the desktop shell never flashes on a phone. `?ui=` is an undocumented, dev-only override
// (spec §8.7) — no preferences-panel toggle exposes it to users.
(function () {
  var params = new URLSearchParams(location.search);
  var override = params.get("ui");
  if (override === "desktop") return;
  var coarse = window.matchMedia("(pointer: coarse)").matches;
  if (override === "touch" || coarse) {
    location.replace("/touch.html" + location.search + location.hash);
  }
})();
