// Touch entry: host-specific refs and the touch shell, handed to the shared `boot()`.
import { boot } from "../boot.js";
import { touchShell } from "./shell.js";
import "./style.css";

void boot(
  {
    root: document.getElementById("form") as HTMLElement,
    openInput: document.getElementById("open") as HTMLInputElement,
    searchInput: document.getElementById("search") as HTMLInputElement,
    results: document.getElementById("results") as HTMLElement,
  },
  touchShell,
);
