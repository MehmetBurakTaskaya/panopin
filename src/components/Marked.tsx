import { highlight } from '../search/highlight';

/** Text with the searched words marked, the way the concept shows them. */
export function Marked({ text, terms, withoutAccents }: { text: string; terms: string[] | null; withoutAccents: boolean }) {
  if (!terms) return <>{text}</>;
  return (
    <>
      {highlight(text, terms, withoutAccents).map((part, i) => (part.hit ? <mark key={i}>{part.text}</mark> : part.text))}
    </>
  );
}
