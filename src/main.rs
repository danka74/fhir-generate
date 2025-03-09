use clap::Parser;
use serde_json::Value;
use std::{
    fs::File,
    io::{BufReader, BufWriter, Write},
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Command
    #[arg(value_enum)]
    command: Commands,

    /// Hide data elements
    #[arg(short, long)]
    elements_hide: bool,

    /// Hide cardinality
    #[arg(short, long)]
    cardinality_hide: bool,

    /// Files to process
    files: Vec<String>,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum Commands {
    PlantUml,
    // Mindmap {},
    // Table {}
}

struct ElementInfo {
    id: String,
    datatype: String,
    min: String,
    max:String,
}

struct DocInfo {
    id: String,
    value: Value,
//    elements: Vec<ElementInfo>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    println!("{:?}", cli);

    let mut docs = Vec::<DocInfo>::new();

    for file in cli.files.iter() {
        if let Ok(value) = load_json_from_file(file) {
            if let Some(id) = value["id"].as_str() {
                let doc = DocInfo {
                    id: id.to_string(),
                    value: value.to_owned(),
                };
                docs.push(doc);
            }
        }
    }

    match cli.command {
        Commands::PlantUml {} => {
            let output = File::create("output.plantuml")?;
            let mut writer = BufWriter::new(output); // Create a buffered writer

            writeln!(
                writer,
                "@startuml\nskinparam linetype polyline\nhide circle\nhide stereotype\nhide methods\n"
            )?;

            for doc in docs.iter() {
                println!("processing: {}", doc.id);
                writeln!(writer, "class **{}** {{", doc.id)?;
                let mut relations = Vec::<(String, String, String, String)>::new();

                if let Some(snapshot) = doc.value["snapshot"]["element"].as_array() {
                    for element in snapshot.iter() {
                        let element_id = element["id"].as_str().ok_or("Missing element id")?;
                        if let Some(element_part) = get_slice_after_last_occurrence(element_id, '.')
                        {
                            let hier_level = count_char_occurrences(element_id, '.') * 2;
                            // extract datatype doc.
                            let mut datatype = element["type"][0]["code"]
                                .as_str()
                                .ok_or("Missing datatype")?;
                            if datatype.starts_with("http") {
                                datatype = get_slice_after_last_occurrence(datatype, '/')
                                    .ok_or("Error in datatype")?;
                            }

                            // extract cardinality min and max values
                            let min = if element["min"].is_string() {
                                element["min"]
                                    .as_str()
                                    .ok_or(format!(
                                        "Missing min cardinality: {:?}",
                                        element["min"]
                                    ))?
                                    .to_string()
                            } else {
                                element["min"].to_string()
                            };
                            let max = if element["max"].is_string() {
                                element["max"]
                                    .as_str()
                                    .ok_or(format!(
                                        "Missing max cardinality: {:?}",
                                        element["max"]
                                    ))?
                                    .to_string()
                            } else {
                                element["max"].to_string()
                            };

                            // if the datatype is one of the classes drawn, add a relation instead of a class element
                            if let Some(_) = docs.iter().position(|s| s.id == datatype) {
                                relations.push((
                                    element_part.to_string(),
                                    datatype.to_string(),
                                    min.to_string(),
                                    max.to_string(),
                                ));
                            } else {
                                if !cli.elements_hide {
                                    write!(
                                        writer,
                                        "{:>hier_level$}|_ {} : {}",
                                        "", element_part, datatype
                                    )?;
                                    if !cli.cardinality_hide {
                                        write!(writer, " [{}..{}]", min, max)?;
                                    }
                                    writeln!(writer)?;
                                }
                            }
                        }
                    }
                }

                writeln!(writer, "}}")?;

                for rel in relations {
                    writeln!(
                        writer,
                        "\"**{}**\" -- \"{}..{}\" \"**{}**\" : {} >",
                        doc.id, rel.2, rel.3, rel.1, rel.0
                    )?;
                }

                writeln!(writer)?;
            }

            writeln!(writer, "@enduml")?;
        }
    }

    Ok(())
}

fn load_json_from_file(path: &String) -> Result<Value, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let value = serde_json::from_reader(reader)?;
    Ok(value)
}

fn get_slice_after_last_occurrence(s: &str, c: char) -> Option<&str> {
    if let Some(last_index) = s.rfind(c) {
        Some(&s[last_index + c.len_utf8()..])
    } else {
        None
    }
}

fn count_char_occurrences(s: &str, c: char) -> usize {
    s.chars().filter(|&ch| ch == c).count()
}
