use assert_cmd::Command;
use std::fs;

#[test]
fn test_plantuml_generation() {
    let mut cmd = Command::cargo_bin("fhir-generate").unwrap();
    cmd.arg("plant-uml")
        .arg("--output-file")
        .arg("output.plantuml")
        .arg("test_data/ps/*.json");

    cmd.assert().success();

    let output = fs::read_to_string("output.plantuml").unwrap();
    assert!(output.contains("@startuml"));
    assert!(output.contains("@enduml"));
}

#[test]
fn test_mindmap_generation() {
    let mut cmd = Command::cargo_bin("fhir-generate").unwrap();
    cmd.arg("mindmap")
        .arg("test_data/StructureDefinition-EHDSAddress.json");

    cmd.assert().success();

    let output = fs::read_to_string("EHDSAddress.plantuml").unwrap();
    assert!(output.contains("@startmindmap"));
    assert!(output.contains("@endmindmap"));
}

#[test]
fn test_table_generation() {
    let mut cmd = Command::cargo_bin("fhir-generate").unwrap();
    cmd.arg("table")
        .arg("test_data/StructureDefinition-EHDSAddress.json");

    cmd.assert().success();

    let output = fs::read_to_string("EHDSAddress.md").unwrap();
    assert!(output.contains("| Code | Element | Short | Definition | Datatype | Cardinality | Preferred Code System | Binding Strength |"));
}
