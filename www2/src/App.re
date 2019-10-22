/* State declaration */

type state = {
  count: int,
  isRunning: bool,
  gameboy: Libation.t,
};

/* Action declaration */
type action =
  | Click
  | Toggle;

let count = 4_190_000;

let gameboy = Libation.createGameboy();
let canvases = Libation.createCanvases();

let document = Webapi.Dom.Document.asEventTarget(Webapi.Dom.document);

[@bs.val] external performanceNow: unit => float = "performance.now";

let getRegisters = () => [
  ("a", Libation.getA(gameboy)),
  ("b", Libation.getB(gameboy)),
  ("c", Libation.getC(gameboy)),
  ("d", Libation.getD(gameboy)),
  ("e", Libation.getE(gameboy)),
  ("h", Libation.getH(gameboy)),
  ("l", Libation.getL(gameboy)),
  ("sp", Libation.getSP(gameboy)),
  ("pc", Libation.getPC(gameboy)),
];

let rec startGameboyLoop = (gameboy, isRunning, registers) => {
  let start = performanceNow();
  /* This should be "proceedToNextVblank" */
  React.Ref.current(isRunning) ?
    Libation.executeOpcodesNoStop(gameboy, 100_000) : ();

  let newRegisters = getRegisters();
  registers->React.Ref.setCurrent(newRegisters);

  let elapsed = performanceNow() -. start;
  let sleepMs =
    React.Ref.current(isRunning) ?
      int_of_float(16.6) - int_of_float(elapsed) : 500;

  ignore(
    Js.Global.setTimeout(
      () => startGameboyLoop(gameboy, isRunning, registers),
      sleepMs,
    ),
  );
};

[@react.component]
let make = () => {
  let isRunning = React.useRef(false);
  let registers = React.useRef(getRegisters());

  let handleKey = event => {
    let key = Webapi.Dom.KeyboardEvent.key(event);
    let key_value =
      switch (key) {
      | "ArrowUp" => 4
      | "ArrowDown" => 8
      | "ArrowLeft" => 2
      | "ArrowRight" => 1
      | "a" => 16
      | "s" => 32
      | "Enter" => 128
      | "Backspace" => 64
      | _ => 16
      };
    Libation.joypadKeyPressed(gameboy, key_value);
    ();
  };
  Webapi.Dom.EventTarget.addKeyDownEventListener(handleKey, document);
  /* .addEventListener("keypress", handleKey, document); */

  let reducer = (state, action) =>
    switch (action) {
    | Click => {...state, count: state.count + 1}
    | Toggle =>
      open React.Ref;
      let newIsRunning = !state.isRunning;
      isRunning->setCurrent(newIsRunning);

      {...state, isRunning: newIsRunning};
    };

  let (_state, dispatch) =
    React.useReducer(
      reducer,
      {
        Libation.start(gameboy);
        Libation.saveGb("gb", gameboy);
        {count: 0, isRunning: isRunning->React.Ref.current, gameboy};
      },
    );

  React.useEffect0(() => {
    startGameboyLoop(gameboy, isRunning, registers);
    None;
  });

  <div>
    <button onClick={_event => dispatch(Toggle)}>
      {ReasonReact.string(isRunning->React.Ref.current ? "Pause" : "Resume")}
    </button>
    <button onClick={_event => Libation.drawObj(canvases, gameboy)}>
      {ReasonReact.string("drawOBJ")}
    </button>
    <Debugger.Registers registers=registers->React.Ref.current />
  </div>;
  /* <Debugger.Breakpoints gameboy /> */
};
