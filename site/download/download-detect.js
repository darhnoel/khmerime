// Download page behaviour. Without this script the page still works: every
// install panel shows, and the main button downloads the Windows installer.
(function () {
  document.documentElement.classList.add("js");

  function detectedOs() {
    var text = (((navigator.userAgentData && navigator.userAgentData.platform) || navigator.platform || "") +
      " " + (navigator.userAgent || "")).toLowerCase();
    if (/android/.test(text)) return "android";
    if (/iphone|ipad|ipod/.test(text) || (/mac/.test(text) && navigator.maxTouchPoints > 1)) return "ios";
    if (/win/.test(text)) return "windows";
    if (/mac/.test(text)) return "macos";
    if (/linux|x11/.test(text)) return "linux";
    return "";
  }

  // Tabs: only the chosen panel shows.
  var tabs = [].slice.call(document.querySelectorAll("[data-tab]"));
  var panels = [].slice.call(document.querySelectorAll("[data-panel]"));
  // The demo follows the tab: phone for Android and iOS, desktop otherwise.
  var video = document.getElementById("demo");
  var figure = video && video.parentNode;
  function showDemo(kind) {
    if (!video || figure.dataset.kind === kind) return;
    figure.dataset.kind = kind;
    video.poster = "assets/demo/" + kind + ".png?v=@DEMO_V@";
    video.src = "assets/demo/" + kind + ".mp4?v=@DEMO_V@";
    if (!reduceMotion) video.play().catch(function () {});
  }
  var reduceMotion = window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  // One underline slides to the chosen tab; panels share one slot, so nothing else moves.
  var ink = document.querySelector(".tab-ink");
  function moveInk(tab) {
    if (!ink || !tab) return;
    ink.style.width = tab.offsetWidth + "px";
    ink.style.transform = "translateX(" + tab.offsetLeft + "px)";
  }
  function show(name) {
    showDemo(name === "android" || name === "ios" ? "phone" : "desktop");
    panels.forEach(function (p) {
      var on = p.dataset.panel === name;
      p.classList.toggle("is-off", !on);
      p.setAttribute("aria-hidden", on ? "false" : "true");
      if (on) p.removeAttribute("inert"); else p.setAttribute("inert", "");
    });
    tabs.forEach(function (t) {
      if (t.dataset.tab === name) { t.setAttribute("aria-current", "true"); moveInk(t); }
      else t.removeAttribute("aria-current");
    });
  }
  window.addEventListener("resize", function () { moveInk(document.querySelector("[data-tab][aria-current]")); });
  if (document.fonts && document.fonts.ready) {
    document.fonts.ready.then(function () { moveInk(document.querySelector("[data-tab][aria-current]")); });
  }
  [].slice.call(document.querySelectorAll("[data-goto]")).forEach(function (link) {
    link.addEventListener("click", function (event) {
      event.preventDefault();
      show(link.dataset.goto);
    });
  });
  tabs.forEach(function (tab) {
    tab.addEventListener("click", function (event) {
      event.preventDefault();
      show(tab.dataset.tab);
    });
  });

  // Main button: this OS's installer if there is one, otherwise the online beta.
  var os = detectedOs();
  show(os || "windows");
  var button = document.getElementById("primary");
  var downloadsOn = document.documentElement.dataset.downloads !== "off";
  if (os === "ios") {
    // iOS installs from the App Store, so this works even while downloads are off.
    document.documentElement.classList.add("store");
    button.href = button.dataset.ios;
    button.removeAttribute("download");
    button.target = "_blank";
    button.rel = "noopener";
    button.innerHTML = 'ទាញយកពី <span lang="en">App Store</span>';
  } else if (!downloadsOn) {
    button.parentNode.hidden = true;
  } else if (os === "linux") {
    button.href = button.dataset.linux;
    button.innerHTML = 'ទាញយកសម្រាប់ <span lang="en">Ubuntu</span>';
  } else if (os === "macos") {
    button.href = button.dataset.macos;
    button.innerHTML = 'ទាញយកសម្រាប់ <span lang="en">macOS</span>';
  } else if (os && os !== "windows") {
    // No installer for this OS yet: its tab explains, so hide the download button.
    button.parentNode.hidden = true;
  }

  // Visitors who ask for less motion get the still poster instead of the loop.
  if (video && reduceMotion) {
    video.removeAttribute("autoplay");
    video.pause();
  }
})();
