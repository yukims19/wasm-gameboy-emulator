module Registers = {
  [@react.component]
  let make = (~registers) => {
    open React;
    open Utils;

    let makeRegister = (~key, name, value) => {
      let bits = 8;
      let padding = bits === 16 ? 5 : 2;
      let toHexPadding = bits === 16 ? 4 : 2;
      let dec = toHex(value, toHexPadding)->string;
      let hex = toPaddedHex(value, padding)->string;
      let bin = toPaddedBinary(value, bits)->string;

      <tr key>
        <td> name->string </td>
        <td> hex </td>
        <td> dec </td>
        <td> bin </td>
      </tr>;
    };

    let registersDisplay =
      registers
      ->Belt.List.mapWithIndex((idx, (name, value)) =>
          makeRegister(~key=string_of_int(idx), name, value)
        )
      ->Array.of_list;

    <ToolPanel name="Registers">
      <table>
        <thead>
          <tr>
            <th> {string("Reg")} </th>
            <th> {string("Hex")} </th>
            <th> {string("Dec")} </th>
            <th> {string("Bin")} </th>
          </tr>
        </thead>
        <tbody> registersDisplay->array </tbody>
      </table>
    </ToolPanel>;
  };
};

module Breakpoints = {
  [@react.component]
  let make = () => {
    open React;
    open Utils;

    let initialBreakPoints = ["00f1", "008b"];
    let (breakPoints, setBreakPoints) = useState(() => initialBreakPoints);
    let (newPoint, setNewPoint) = useState(() => "");

    /* let breakpoints = gameboy->Libation.getBreakPoints; */

    /* let breakpoints = */
    /*   breakpoints */
    /*   ->Belt.Array.map(address => <li> {string(toHex(address, 6))} </li>); */

    let handleKeyPress = key =>
      if (key == "Enter" && newPoint != "") {
        let newBreakPoints = breakPoints->List.append([newPoint]);
        setBreakPoints(_ => newBreakPoints);
        setNewPoint(_ => "");
      };

    let handleClick = target => {
      if (target##checked) {
        Libation.setBreakPoint(target##value);
      } else {
        Libation.removeBreakPoint(target##value);
      };
      ();
    };

    let breakPointsChecker =
      breakPoints
      ->Belt.List.mapWithIndex((idx, point) =>
          <div>
            <span key={string_of_int(idx)}>
              <input
                type_="checkbox"
                value=point
                name=point
                onClick={
                  event => handleClick(ReactEvent.Mouse.target(event))
                }
              />
            </span>
            <span> {string(point)} </span>
          </div>
        )
      ->Array.of_list;

    <ToolPanel name="PC Breakpoints">
      <input
        placeholder="New Break Point"
        value=newPoint
        onKeyPress={event => handleKeyPress(ReactEvent.Keyboard.key(event))}
        onChange={event => setNewPoint(ReactEvent.Form.target(event)##value)}
      />
      <div> breakPointsChecker->array </div>
    </ToolPanel>;
  };
};

/* <ul> */
/* <li> */
/* <button onClick={_ => gameboy->Libation.setBreakPoint(0x2000)}> */
/*   {string("Toggle breakpoint")} */
/*           </button> */
/*         </li> */
/*         breakpoints->array */
/*       </ul> */
