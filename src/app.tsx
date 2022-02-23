/* eslint-disable no-redeclare */
import { isRegistered, register } from "@tauri-apps/api/globalShortcut";
import { Component, h } from "preact";
import { invoke } from "@tauri-apps/api/tauri";
import { clipboard } from "@tauri-apps/api";

type Props = {};
type State = {
  count: number;
};

class App extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = {
      count: 0,
    };
    this.handleCtrlD = this.handleCtrlD.bind(this);
  }

  handleCtrlD(): void {
    console.log("Ctrl+D pressed");
    clipboard.writeText("");
    invoke("send_adv_copy").then(() => {
      clipboard.readText().then((text) => {
        console.log("text copied", text);
      });
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
    const { count } = this.state;
    return (
      <>
        <p>Hello Vite + Preact! {count}</p>
        <p>
          <a
            class="link"
            href="https://preactjs.com/"
            target="_blank"
            rel="noopener noreferrer"
          >
            Learn Preact
          </a>
        </p>
      </>
    );
  }
}

export default App;
