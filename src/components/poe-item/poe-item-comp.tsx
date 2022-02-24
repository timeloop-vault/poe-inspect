import { PoeItem } from "@timeloop-vault/poe-item/dist/poe-item-types";
import { h } from "preact";

type Props = {
  poeItem: PoeItem;
};

function PoeItemComp({ poeItem }: Props): h.JSX.Element {
  return (
    <div>
      <p>{poeItem.name}</p>
      <p>{poeItem.rarity}</p>
      <p>{poeItem.itemClass}</p>
    </div>
  );
}

export default PoeItemComp;
