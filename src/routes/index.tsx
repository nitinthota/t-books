import { createFileRoute } from "@tanstack/react-router";
import { TBooksRoot } from "@/components/t-books/t-books-root";

export const Route = createFileRoute("/")({
  component: Home,
});

function Home() {
  return <TBooksRoot />;
}
