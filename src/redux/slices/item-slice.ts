import { createSlice, PayloadAction } from "@reduxjs/toolkit";
import { Rarities } from "@timeloop-vault/poe-item/dist/poe-item-enums";
import { Range } from "@timeloop-vault/poe-item/dist/poe-item-types";
import type { RootState } from "../store/store";

// Define a type for the slice state
interface ItemState {
  itemMatches: ItemMatch[];
}

// Define the initial state using that type
const initialState: ItemState = {
  itemMatches: [],
};

export type ItemMatch = {
  class: string;
  base: string | null;
  rarity: Rarities | null;
  implicit: ItemMatchMod[] | null;
  prefix: ItemMatchMod[] | null;
  suffix: ItemMatchMod[] | null;
  enchant: ItemMatchMod[] | null;
  unique: ItemMatchMod[] | null;
};

export type ItemMatchMod = {
  minTier: number | null;
  maxTier: number | null;
  valueRanges: Range[];
  crafted: boolean;
  fractured: boolean;
  tierRangesIndexText: string;
};

export const itemSlice = createSlice({
  name: "item",
  // `createSlice` will infer the state type from the `initialState` argument
  initialState,
  reducers: {
    addItemMatch: (state, action: PayloadAction<ItemMatch>) => {
      console.log("addItemMatch", action.payload);
      state.itemMatches = [...state.itemMatches, action.payload];
    },
  },
});

export const { addItemMatch } = itemSlice.actions;

// Other code such as selectors can use the imported `RootState` type
export const itemMatches = (state: RootState) => state.item.itemMatches;

export default itemSlice.reducer;
