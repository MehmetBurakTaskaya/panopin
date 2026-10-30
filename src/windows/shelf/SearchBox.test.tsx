import { cleanup, render, screen } from '@testing-library/react';
import { createRef } from 'react';
import { afterEach, describe, expect, it } from 'vitest';
import { SearchBox } from './SearchBox';

describe('SearchBox', () => {
  afterEach(cleanup);

  it('takes the focus as it appears, so typing never reaches the letter keys', () => {
    render(
      <SearchBox
        inputRef={createRef()}
        query=""
        counts={null}
        hidden={{ image: false, text: false, link: false }}
        withoutAccents={false}
        top={200}
        onQuery={() => {}}
        onAccents={() => {}}
        onResults={() => {}}
        onClose={() => {}}
      />,
    );
    expect(document.activeElement).toBe(screen.getByRole('searchbox'));
  });
});
