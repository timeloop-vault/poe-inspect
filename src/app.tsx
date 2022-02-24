/* eslint-disable no-redeclare */
import { isRegistered, register } from "@tauri-apps/api/globalShortcut";
import { Component, h } from "preact";
import { invoke } from "@tauri-apps/api/tauri";
import { clipboard } from "@tauri-apps/api";
import { PoeItem } from "@timeloop-vault/poe-item/dist/poe-item-types";
import { item } from "@timeloop-vault/poe-item/dist/poe-item";
import PoeItemComp from "./components/poe-item/poe-item-comp";

type Props = {};
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
  }

  clipboardReadText(): void {
    clipboard.readText().then((text) => {
      console.log("text copied", text);
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

  handleCtrlD(): void {
    console.log("Ctrl+D pressed");
    clipboard.writeText("");
    invoke("send_adv_copy").then(() => {
      this.clipboardReadText();
    });
    const { count } = this.state;
    this.setState({
      count: count + 1,
    });
  }

  componentDidMount(): void {
    isRegistered("Ctrl+D")
      .then((registered) => {
        if (!registered) {
          register("Ctrl+D", this.handleCtrlD);
        }
      })
      .catch((err) => {
        console.error(err);
      });
  }

  render(): h.JSX.Element {
    const { poeItem } = this.state;
    return (
      <>
        <p>PoE Inspect</p>
        <p>{poeItem && <PoeItemComp poeItem={poeItem} />}</p>
      </>
    );
  }
}

export default App;
