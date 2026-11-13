// Reads the lcov.info written by `cargo tarpaulin --out lcov` and groups it by crate.
// Shared by the landing page and the coverage dashboard.

"use strict";

const RSnesCoverage = (() => {
    // Minimum workspace coverage required by CONTRIBUTING.md, also shown as a target for each crate
    const THRESHOLD = 80;

    function parseLcov(text) {
        const files = [];
        let cur = null;

        for (const raw of text.split(/\r?\n/)) {
            const line = raw.trim();
            if (line.startsWith("SF:")) {
                cur = { path: line.slice(3).replace(/\\/g, "/"), daFound: 0, daHit: 0 };
            } else if (!cur) {
                continue;
            } else if (line.startsWith("DA:")) {
                cur.daFound++;
                if (Number(line.split(",")[1]) > 0) cur.daHit++;
            } else if (line.startsWith("LF:")) {
                cur.lf = Number(line.slice(3));
            } else if (line.startsWith("LH:")) {
                cur.lh = Number(line.slice(3));
            } else if (line === "end_of_record") {
                files.push({
                    path: cur.path,
                    found: cur.lf ?? cur.daFound,
                    hit: cur.lh ?? cur.daHit,
                });
                cur = null;
            }
        }
        return files.filter((f) => f.found > 0);
    }

    // Tarpaulin writes absolute paths: strip the directory they all share,
    // which is the workspace root.
    function stripCommonDir(files) {
        if (files.length < 2) return files;
        const split = files.map((f) => f.path.split("/"));
        let common = 0;
        while (split.every((s) => common < s.length - 1 && s[common] === split[0][common])) {
            common++;
        }
        return files.map((f, i) => ({ ...f, path: split[i].slice(common).join("/") }));
    }

    // A file belongs to the crate whose folder holds its `src/` directory
    // (`cpu/instr_metalang_procmacro/src/lib.rs` -> `cpu/instr_metalang_procmacro`).
    function crateDir(path) {
        const parts = path.split("/");
        const src = parts.indexOf("src");
        return (src >= 0 ? parts.slice(0, src) : parts.slice(0, -1)).join("/");
    }

    function crateName(dir) {
        return dir === "" ? "r-snes" : dir.split("/").pop();
    }

    function percent(hit, found) {
        return found === 0 ? 0 : (hit / found) * 100;
    }

    function summarize(lcovText) {
        const files = stripCommonDir(parseLcov(lcovText));
        const crates = new Map();

        for (const f of files) {
            const dir = crateDir(f.path);
            if (!crates.has(dir)) {
                crates.set(dir, { dir, name: crateName(dir), hit: 0, found: 0, files: [] });
            }
            const c = crates.get(dir);
            c.hit += f.hit;
            c.found += f.found;
            c.files.push({ ...f, pct: percent(f.hit, f.found) });
        }

        const list = [...crates.values()];
        for (const c of list) {
            c.pct = percent(c.hit, c.found);
            c.files.sort((a, b) => a.path.localeCompare(b.path));
        }
        list.sort((a, b) => a.name.localeCompare(b.name));

        const hit = list.reduce((n, c) => n + c.hit, 0);
        const found = list.reduce((n, c) => n + c.found, 0);
        return { hit, found, pct: percent(hit, found), crates: list };
    }

    async function load(url) {
        const res = await fetch(url, { cache: "no-cache" });
        if (!res.ok) throw new Error(`${url}: HTTP ${res.status}`);
        return summarize(await res.text());
    }

    const ICON_OK =
        '<svg viewBox="0 0 16 16" fill="currentColor" aria-hidden="true"><path d="M8 0a8 8 0 1 1 0 16A8 8 0 0 1 8 0Zm3.4 5.1a.75.75 0 0 0-1.06.02L7.1 8.6 5.66 7.1a.75.75 0 1 0-1.08 1.04l2 2.08a.75.75 0 0 0 1.08 0l3.76-4.06a.75.75 0 0 0-.02-1.06Z"/></svg>';
    const ICON_WARN =
        '<svg viewBox="0 0 16 16" fill="currentColor" aria-hidden="true"><path d="M6.46 1.1a1.75 1.75 0 0 1 3.08 0l6.2 11.36A1.75 1.75 0 0 1 14.2 15H1.8a1.75 1.75 0 0 1-1.54-2.54ZM8 5a.75.75 0 0 0-.75.75v3.5a.75.75 0 0 0 1.5 0v-3.5A.75.75 0 0 0 8 5Zm0 7.5a1 1 0 1 0 0-2 1 1 0 0 0 0 2Z"/></svg>';

    function statusHtml(pct) {
        return pct >= THRESHOLD
            ? `<span class="status good" title="Meets the ${THRESHOLD}% minimum">${ICON_OK}<span class="status-label">Meets ${THRESHOLD}%</span></span>`
            : `<span class="status warn" title="Below the ${THRESHOLD}% minimum">${ICON_WARN}<span class="status-label">Below ${THRESHOLD}%</span></span>`;
    }

    function meterHtml(pct, label) {
        const w = Math.max(0, Math.min(100, pct)).toFixed(1);
        return `<div class="meter" role="img" aria-label="${label}: ${pct.toFixed(1)}%" title="${label}: ${pct.toFixed(1)}%"><span style="width:${w}%"></span></div>`;
    }

    return { THRESHOLD, load, summarize, statusHtml, meterHtml };
})();
