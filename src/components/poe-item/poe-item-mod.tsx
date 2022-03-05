import { Mod } from "@timeloop-vault/poe-item/dist/poe-item-types";
import { h } from "preact";
import { parseNumber } from "../../utils";

type Props = {
  mod: Mod;
};

const tierIndex = /{([0-9]+)}/g;

function PoeItemMod({ mod }: Props): h.JSX.Element {
  console.log(mod.tierRangesIndexText);
  const matches = mod.tierRangesIndexText.matchAll(tierIndex);
  let text = mod.tierRangesIndexText;
  if (matches) {
    for (const match of matches) {
      console.log(match);
      if (match == undefined || match.index == undefined) {
        continue;
      }

      const index = parseNumber(match[1]);
      if (index == null) {
        continue;
      }

      const range = mod.tierRanges[index];
      text = text.replace(
        match[0],
        `${range.value}(${range.min}-${range.max})`
      );
    }
  }
  if (mod.groups != undefined) {
    text = `${text} (${mod.groups.join(", ")})`;
  }
  return (
    <div>
      <p>{text}</p>
    </div>
  );
}

export default PoeItemMod;
