import { createSignal, onMount } from "solid-js";
import { invoke } from "@tauri-apps/api/tauri";
import { emit, listen } from '@tauri-apps/api/event'
import "./App.css";

function App() {
  const [greetMsg, setGreetMsg] = createSignal("");
  const [env, setEnv] = createSignal("local");
  onMount(() => {
    listen('sk_lifecycle_events', (e) => {
      console.log(e)
    })
  })
  async function start_sk() {
    // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
    setGreetMsg(await invoke("start_sk", { env: env() }));

    
  }

  return (
    <div class="container">
      {/* <h1>Welcome to skn!</h1>
      <div class="row">
        <img src="/logo.png" class="logo solid" alt="Solid logo" />
      </div> */}
      <p>Click to start node.</p>

      <div class="row">
        <div>
          <input
            id="greet-input"
            value={env()}
            onChange={(e) => setEnv(e.currentTarget.value)}
            placeholder="Enter a env..."
          />
          <button type="button" onClick={() => start_sk()}>
            Greet
          </button>
        </div>
      </div>

      <p>{greetMsg}</p>
    </div>
  );
}

export default App;
