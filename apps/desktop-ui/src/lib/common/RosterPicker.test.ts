// @vitest-environment happy-dom
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import RosterPicker from './RosterPicker.svelte';
afterEach(cleanup);
describe('RosterPicker',()=>{it('opens with ArrowDown, selects with Enter, and closes with Escape',async()=>{const onSelect=vi.fn();render(RosterPicker,{roster:[{personId:1,canonicalName:'Ada Lovelace',pinyin:'ada'}],onSelect});const input=screen.getByRole('combobox');await fireEvent.keyDown(input,{key:'ArrowDown'});expect(screen.getByRole('listbox')).toBeTruthy();await fireEvent.keyDown(input,{key:'Enter'});expect(onSelect).toHaveBeenCalledWith(1);await fireEvent.keyDown(input,{key:'ArrowDown'});await fireEvent.keyDown(input,{key:'Escape'});expect(screen.queryByRole('listbox')).toBeNull();});it('shows No results for an unmatched query',async()=>{render(RosterPicker,{roster:[]});await fireEvent.focus(screen.getByRole('combobox'));expect(screen.getByText('No results')).toBeTruthy();});});
