// Adds a button leading back to the R-SNES website in the menu bar.
(() => {
    const buttons = document.querySelector("#mdbook-menu-bar .right-buttons");
    if (!buttons) return;

    const home = document.createElement("a");
    home.href = `${typeof path_to_root === "string" ? path_to_root : ""}../index.html`;
    home.title = "R-SNES website";
    home.setAttribute("aria-label", "R-SNES website");
    home.innerHTML =
        '<span class="fa-svg"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="M3 10.5 12 3l9 7.5"/><path d="M5 9v12h14V9"/><path d="M10 21v-6h4v6"/></svg></span>';
    buttons.prepend(home);
})();
