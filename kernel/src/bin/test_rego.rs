use regorus::{Engine, Value};

fn main() {
    let mut engine = Engine::new();
    engine.add_policy("hipaa.rego".to_string(), r#"
package ucser.execution

default allow = false

allow {
    not is_restricted_command
}

is_restricted_command {
    restricted_commands := {"rm", "del"}
    input.command == restricted_commands[_]
}
    "#.to_string()).unwrap();

    let input_str = r#"{"command": "ls"}"#;
    let input_val = Value::from_json_str(input_str).unwrap();
    engine.set_input(input_val);

    let res = engine.eval_query("data.ucser.execution.allow".to_string(), false).unwrap();
    println!("Result ls: {:?}", res);

    let input_str2 = r#"{"command": "rm"}"#;
    let input_val2 = Value::from_json_str(input_str2).unwrap();
    engine.set_input(input_val2);

    let res2 = engine.eval_query("data.ucser.execution.allow".to_string(), false).unwrap();
    println!("Result rm: {:?}", res2);
}
