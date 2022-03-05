import { Mod } from "@timeloop-vault/poe-item/dist/poe-item-types";
import { h } from "preact";
import PoeItemMod from "./poe-item-mod";

type Props = {
  mods: Mod[];
};

function PoeItemUnique({ mods }: Props): h.JSX.Element {
  return (
    <div>
      <p>
        {mods.map((mod, index) => {
          return <PoeItemMod key={index} mod={mod} />;
        })}
      </p>
    </div>
  );
}

export default PoeItemUnique;
