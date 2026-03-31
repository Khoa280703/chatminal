document.documentElement.classList.add("js");

const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
const revealElements = document.querySelectorAll("[data-reveal]");

if ("IntersectionObserver" in window && !reduceMotion) {
  const revealObserver = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (!entry.isIntersecting) return;
        entry.target.classList.add("is-visible");
        revealObserver.unobserve(entry.target);
      });
    },
    { threshold: 0.16, rootMargin: "0px 0px -8% 0px" },
  );

  revealElements.forEach((element) => revealObserver.observe(element));
} else {
  revealElements.forEach((element) => element.classList.add("is-visible"));
}

if (!reduceMotion) {
  const parallaxElements = document.querySelectorAll("[data-parallax]");
  let ticking = false;

  const updateParallax = () => {
    const scrollTop = window.scrollY;

    parallaxElements.forEach((element) => {
      const depth = Number(element.getAttribute("data-parallax")) || 18;
      element.style.setProperty("--parallax-offset", `${scrollTop / depth}px`);
    });

    ticking = false;
  };

  updateParallax();

  window.addEventListener(
    "scroll",
    () => {
      if (ticking) return;
      window.requestAnimationFrame(updateParallax);
      ticking = true;
    },
    { passive: true },
  );
}
