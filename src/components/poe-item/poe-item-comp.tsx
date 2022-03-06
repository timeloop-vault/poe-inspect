import { PoeItem } from "@timeloop-vault/poe-item/dist/poe-item-types";
import { h } from "preact";
import PoeItemEnchant from "./poe-item-enchant";
import PoeItemImplicit from "./poe-item-implicit";
import PoeItemPrefix from "./poe-item-prefix";
import PoeItemSuffix from "./poe-item-suffix";
import PoeItemUnique from "./poe-item-unique";

type Props = {
  poeItem: PoeItem;
};

function PoeItemComp({ poeItem }: Props): h.JSX.Element {
  return (
    <div>
      <p>General</p>
      <p>Class: {poeItem.itemClass}</p>
      <p>Base: {poeItem.itemBase}</p>
      <p>Rarity: {poeItem.rarity}</p>
      {poeItem.enchants.length > 0 && (
        <PoeItemEnchant mods={poeItem.enchants} />
      )}
      {poeItem.implicits.length > 0 && (
        <PoeItemImplicit mods={poeItem.implicits} />
      )}
      {poeItem.prefixes.length > 0 && <PoeItemPrefix mods={poeItem.prefixes} />}
      {poeItem.suffixes.length > 0 && <PoeItemSuffix mods={poeItem.suffixes} />}
      {poeItem.uniques.length > 0 && <PoeItemUnique mods={poeItem.uniques} />}
    </div>
  );
}

export default PoeItemComp;
