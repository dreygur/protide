"use client";

import { useEffect } from "react";

/** Reveals `[data-animate]` elements as they scroll into view (see globals.css). */
export default function ScrollReveal() {
  useEffect(() => {
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            entry.target.classList.add("in-view");
            observer.unobserve(entry.target);
          }
        }
      },
      { rootMargin: "0px 0px -48px 0px", threshold: 0.1 },
    );

    for (const el of document.querySelectorAll(".landing [data-animate]")) {
      observer.observe(el);
    }
    return () => observer.disconnect();
  }, []);

  return null;
}
