import { Mod } from "@timeloop-vault/poe-item/dist/poe-item-types";
import { h } from "preact";
import PoeItemMod from "./poe-item-mod";

type Props = {
  mods: Mod[];
};

function PoeItemPrefix({ mods }: Props): h.JSX.Element {
  return (
    <div>
      <p>Prefix</p>
      {mods.map((mod, index) => {
        return <PoeItemMod key={index} mod={mod} />;
      })}
    </div>
  );
}

export default PoeItemPrefix;
