// Behaviour shared by the hand-written pages: theme toggle, mobile menu,
// nav highlight and the "built from commit" footer line.

"use strict";

(() => {
    const root = document.documentElement;

    // ---------- Theme toggle (remembered per browser, restored in each page's <head>) ----------
    const toggle = document.querySelector("[data-theme-toggle]");
    toggle?.addEventListener("click", () => {
        const dark =
            root.dataset.theme === "dark" ||
            (!root.dataset.theme && matchMedia("(prefers-color-scheme: dark)").matches);
        root.dataset.theme = dark ? "light" : "dark";
        try {
            localStorage.setItem("rsnes-theme", root.dataset.theme);
        } catch {
            // not remembered, still applied
        }
    });

    // ---------- Mobile menu ----------
    const nav = document.querySelector(".nav");
    const menu = document.querySelector(".menu-toggle");
    menu?.addEventListener("click", () => {
        const open = nav.classList.toggle("open");
        menu.setAttribute("aria-expanded", String(open));
    });
    nav?.addEventListener("click", (e) => {
        if (e.target.closest("a")) nav.classList.remove("open");
    });

    // ---------- Highlight the nav link of the section on screen ----------
    if ("IntersectionObserver" in window) {
        const links = new Map(
            [...document.querySelectorAll('.nav a[href^="#"]')].map((a) => [a.hash.slice(1), a]),
        );
        const spy = new IntersectionObserver(
            (entries) => {
                for (const e of entries) {
                    if (e.isIntersecting) {
                        links.forEach((a) => a.classList.remove("active"));
                        links.get(e.target.id)?.classList.add("active");
                    }
                }
            },
            { rootMargin: "-45% 0px -50% 0px" },
        );
        links.forEach((_, id) => {
            const section = document.getElementById(id);
            if (section) spy.observe(section);
        });
    }

    // ---------- Build information (written by site/build.sh) ----------
    const info = document.querySelector("[data-build-info]");
    if (info) {
        fetch(info.dataset.buildInfo, { cache: "no-cache" })
            .then((res) => (res.ok ? res.json() : Promise.reject()))
            .then(({ commit, date }) => {
                const a = document.createElement("a");
                a.href = `https://github.com/r-snes/r-snes/commit/${commit}`;
                a.textContent = commit.slice(0, 7);
                info.append("Built from ", a, ` on ${date}.`);
            })
            .catch(() => info.remove());
    }
})();
