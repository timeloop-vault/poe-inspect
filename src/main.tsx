import { render } from "preact";
import { Provider } from "react-redux";
import App from "./app";
import "./index.css";
import { store } from "./redux/store/store";

render(
  <Provider store={store}>
    <App />
  </Provider>,
  document.getElementById("app")!
);
