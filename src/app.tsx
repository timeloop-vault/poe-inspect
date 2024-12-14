import { register, isRegistered, ShortcutEvent } from '@tauri-apps/plugin-global-shortcut';
import { Component, h } from "preact";
import { invoke } from '@tauri-apps/api/core';
import { readText, writeText } from '@tauri-apps/plugin-clipboard-manager';
import { Mod, PoeItem } from "@timeloop-vault/poe-item/dist/poe-item-types";
import { item } from "@timeloop-vault/poe-item/dist/poe-item";
import PoeItemComp from "./components/poe-item/poe-item-comp";
import { info, error } from '@tauri-apps/plugin-log';

import {
  addItemMatch,
  ItemMatch,
  ItemMatchMod,
} from "./redux/slices/item-slice";
import { RootState } from "./redux/store/store";
import { connect, ConnectedProps } from "react-redux";

const mapState = (state: RootState) => ({
  itemMatches: state.item.itemMatches,
});

const mapDispatch = {
  addItemMatch,
};

const connector = connect(mapState, mapDispatch);

type PropsFromRedux = ConnectedProps<typeof connector>;

type Props = PropsFromRedux & {};
type State = {
  count: number;
  poeItem: PoeItem | null;
};

class App extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = {
      count: 0,
      poeItem: null,
    };
    this.handleCtrlD = this.handleCtrlD.bind(this);
    this.clipboardReadText = this.clipboardReadText.bind(this);
    this.onSave = this.onSave.bind(this);
  }

  clipboardReadText(): void {
    readText().then((text) => {
      info(`text copied: ${text}`);
      if (text != null && text.length > 0) {
        const poeItem = item(text, true);
        this.setState({
          poeItem,
        });
      } else {
        setTimeout(() => {
          this.clipboardReadText();
        }, 1000);
      }
    });
  }

  handleCtrlD(event: ShortcutEvent): void {
    if (event.state === "Released") {
      info("Ctrl+D pressed");
      writeText("");
      invoke("send_adv_copy").then(() => {
        this.clipboardReadText();
      });
      const { count } = this.state;
      this.setState({
        count: count + 1,
      });
    }
  }

  componentDidMount(): void {
    isRegistered("Ctrl+S")
      .then((registered) => {
        if (!registered) {
          register("Ctrl+S", this.handleCtrlD);
        }
      })
      .catch((err) => {
        error("Unable to setup Global Shortcuts", err);
      });
  }

  convertMod(mod: Mod): ItemMatchMod {
    return {
      minTier: mod.tier || null,
      maxTier: mod.tier || null,
      crafted: mod.crafted || false,
      fractured: mod.fractured || false,
      tierRangesIndexText: mod.tierRangesIndexText || "",
      valueRanges: mod.tierRanges,
    };
  }

  onSave(): void {
    const { poeItem } = this.state;
    const { addItemMatch } = this.props;
    if (poeItem) {
      const itemMatch: ItemMatch = {
        class: poeItem.itemClass || "",
        base: poeItem.itemBase || null,
        rarity: poeItem.rarity || null,
        implicit: poeItem.implicits.map((mod) => this.convertMod(mod)),
        prefix: poeItem.prefixes.map((mod) => this.convertMod(mod)),
        suffix: poeItem.suffixes.map((mod) => this.convertMod(mod)),
        unique: poeItem.uniques.map((mod) => this.convertMod(mod)),
        enchant: poeItem.enchants.map((mod) => this.convertMod(mod)),
      };
      info(JSON.stringify(itemMatch));
      addItemMatch(itemMatch);
    }
    info("save");
  }

  render(): h.JSX.Element {
    const { poeItem } = this.state;
    return (
      <>
        <p>PoE Inspect</p>
        <button onClick={this.onSave}>Save</button>
        <p>{poeItem && <PoeItemComp poeItem={poeItem} />}</p>
      </>
    );
  }
}

export default connector(App);
