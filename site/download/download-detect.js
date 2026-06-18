(function () {
  function platformText() {
    if (navigator.userAgentData && navigator.userAgentData.platform) {
      return navigator.userAgentData.platform;
    }
    return navigator.platform || navigator.userAgent || "";
  }

  function detectedPlatform() {
    var platform = platformText().toLowerCase();
    if (platform.indexOf("win") !== -1) {
      return "windows";
    }
    if (platform.indexOf("linux") !== -1 || platform.indexOf("ubuntu") !== -1) {
      return "linux";
    }
    if (platform.indexOf("mac") !== -1) {
      return "macos";
    }
    return "";
  }

  function promoteDownload(platform) {
    if (!platform) {
      return;
    }

    var buttons = document.querySelectorAll("[data-download-platform]");
    buttons.forEach(function (button) {
      var isPrimary = button.dataset.downloadPlatform === platform;
      button.classList.toggle("primary", isPrimary);
      button.classList.toggle("secondary", !isPrimary);
      button.style.order = isPrimary ? "0" : "1";
    });
  }

  function hideScrollHintOnScroll() {
    var hint = document.querySelector(".scroll-hint");
    if (!hint) return;

    var hasScrolled = false;
    window.addEventListener(
      "scroll",
      function () {
        if (!hasScrolled && window.scrollY > 50) {
          hint.style.animation = "none";
          hint.style.opacity = "0";
          hint.style.pointerEvents = "none";
          hasScrolled = true;
        }
      },
      { once: true }
    );
  }

  promoteDownload(detectedPlatform());
  hideScrollHintOnScroll();
})();

