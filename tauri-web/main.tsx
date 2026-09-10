import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { TBooksRoot } from "../src/components/t-books/t-books-root";
import "../src/styles.css";

const root = document.getElementById("app");
if (!root) {
  throw new Error("T Books could not find the app root.");
}

createRoot(root).render(
  <StrictMode>
    <TBooksRoot />
  </StrictMode>,
);
