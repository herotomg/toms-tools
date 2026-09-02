/* Share button for the persistent header.
 *
 * Always copies to the clipboard rather than opening the OS share sheet:
 * the job here is "give me a link to paste to a teammate", and a share sheet
 * is a slower path to the same string. Copies whatever the address bar holds,
 * so sharing while a comment is open hands over a link to that comment. */
(() => {
  const btn = document.querySelector(".ta-share");
  if (!btn) return;

  function legacyCopy(text) {
    const box = document.createElement("textarea");
    box.value = text;
    box.style.cssText = "position:fixed;top:0;left:0;opacity:0";
    document.body.appendChild(box);
    box.select();
    try { return document.execCommand("copy"); }
    catch { return false; }
    finally { box.remove(); }
  }

  btn.addEventListener("click", async () => {
    const url = location.href;
    let ok = false;
    try {
      await navigator.clipboard.writeText(url);
      ok = true;
    } catch {
      ok = legacyCopy(url);
    }
    const was = btn.dataset.label || btn.textContent;
    btn.dataset.label = was;
    // Never open a modal here: a blocking prompt() freezes the page, which is
    // a far worse outcome than a copy that quietly did not take.
    btn.textContent = ok ? "Link copied" : "Copy failed";
    btn.classList.toggle("ta-done", ok);
    setTimeout(() => {
      btn.textContent = was;
      btn.classList.remove("ta-done");
    }, 1600);
  });
})();
