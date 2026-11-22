// @vitest-environment happy-dom
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import DataTable from './DataTable.svelte';

describe('DataTable', () => {
  afterEach(cleanup);
  const items = [{ id: 'b', name: 'Bee', count: 2 }, { id: 'a', name: 'Ada', count: 1 }];
  const columns = [{ key: 'name', label: 'Name' }, { key: 'count', label: 'Count' }];
  it('sorts rows when a column header is selected', async () => {
    render(DataTable, { props: { items, columns, emptyLabel: 'Nothing here', selectAllLabel: 'Select all' } });
    const nameHeader = screen.getByRole('button', { name: 'Name' });
    await fireEvent.click(nameHeader);
    const cells = screen.getAllByRole('button').filter((button) => button.classList.contains('cell'));
    expect(cells[0].textContent).toBe('Ada');
    await fireEvent.click(nameHeader);
    const reverse = screen.getAllByRole('button').filter((button) => button.classList.contains('cell'));
    expect(reverse[0].textContent).toBe('Bee');
  });
  it('reports individual and select all changes', async () => {
    const onSelectionChange = vi.fn();
    render(DataTable, { props: { items, columns, selectedIds: [], emptyLabel: 'Nothing here', selectAllLabel: 'Select all', onSelectionChange } });
    await fireEvent.click(screen.getByRole('checkbox', { name: 'a' }));
    expect(onSelectionChange).toHaveBeenLastCalledWith(['a']);
    await fireEvent.click(screen.getByRole('checkbox', { name: 'Select all' }));
    expect(onSelectionChange).toHaveBeenLastCalledWith(['b', 'a']);
  });
  it('keeps the full meeting topic in the topic cell title', () => {
    const topic = 'A meeting topic long enough to be truncated in the records table';
    render(DataTable, { props: { items: [{ id: 'topic', topic }], columns: [{ key: 'topic', label: 'Meeting', value: (row: any) => row.topic }] } });
    expect(screen.getByRole('button', { name: topic }).getAttribute('title')).toBe(topic);
  });
});
