import { configureStore } from "@reduxjs/toolkit";
import itemReducer from "../slices/item-slice";
import devToolsEnhancer from "remote-redux-devtools";

export const store = configureStore({
  reducer: {
    item: itemReducer,
  },
  enhancers: [devToolsEnhancer({ realtime: true, port: 8000, secure: false })],
});

// Infer the `RootState` and `AppDispatch` types from the store itself
export type RootState = ReturnType<typeof store.getState>;
// Inferred type: {posts: PostsState, comments: CommentsState, users: UsersState}
export type AppDispatch = typeof store.dispatch;
