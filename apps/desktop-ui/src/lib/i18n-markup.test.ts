import { describe, expect, it } from "vitest";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const componentDir = join(process.cwd(), "src");
const pagesDir = join(componentDir, "pages");
const commonDir = join(componentDir, "lib", "common");
const pageFiles: string[] = [];
function collectPageFiles(directory: string) {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const file = join(directory, entry.name);
    if (entry.isDirectory()) collectPageFiles(file);
    else if (entry.name.endsWith(".svelte")) pageFiles.push(file);
  }
}
collectPageFiles(pagesDir);
collectPageFiles(commonDir);
const files = [
  join(componentDir, "App.svelte"),
  ...readdirSync(join(componentDir, "lib"))
    .filter((name) => name.endsWith(".svelte"))
    .map((name) => join(componentDir, "lib", name)),
  ...pageFiles,
];
const allowed =
  /^(AttendFlow|AttendFlow Desktop|Zoom|CSV|JSON|ID|English|简体中文|DESKTOP|Zoom Companion|[↑↓×◉⌘⚙▤⇄A])$/;

describe("Svelte markup localization", () => {
  it("keeps English text nodes out of markup unless explicitly allow-listed", () => {
    const findings: string[] = [];
    for (const file of files) {
      const source = readFileSync(file, "utf8").replace(
        /<script[\s\S]*?<\/script>|<style[\s\S]*?<\/style>/g,
        "",
      );
      for (const match of source.matchAll(
        />(([A-Za-z][A-Za-z0-9 #&·,.-]{2,}))</g,
      )) {
        const text = match[1].replace(/\s+/g, " ").trim();
        if (!text || allowed.test(text) || !/[A-Za-z]{3}/.test(text)) continue;
        findings.push(`${file}: ${text}`);
      }
    }
    expect(findings, findings.join("\n")).toEqual([]);
  });
});
