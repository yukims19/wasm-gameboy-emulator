[@react.component]
let make = (~name, ~children) => {
  open React;
  let (isOpen, setOpen) = useState(() => true);
  <div>
    <h3 onClick={_ => setOpen(isOpen => !isOpen)}> {React.string(name)} </h3>
    {isOpen ? children : null}
  </div>;
};
