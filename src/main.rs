use clap::{Args, Parser, Subcommand};
use serde_json::Value;
use std::{
    fs::File,
    io::{BufReader, BufWriter, Write},
    path::PathBuf,
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Command
    #[command(subcommand)]
    command: Commands,
}

#[derive(Args, Debug)]
struct CommonArgs {
    /// Hide data elements
    #[arg(short, long)]
    elements_hide: bool,

    /// Hide cardinality
    #[arg(short, long)]
    cardinality_hide: bool,

    /// Files to process
    files: Vec<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    PlantUml(PlantUmlArgs),
    Mindmap(MindmapArgs),
    // Table {}
}

#[derive(Args, Debug)]
struct PlantUmlArgs {
    #[command(flatten)]
    common: CommonArgs,

    #[arg(short, long, default_value = "output.plantuml")]
    output_file: PathBuf,
}

#[derive(Args, Debug)]
struct MindmapArgs {
    #[command(flatten)]
    common: CommonArgs,

    #[arg(short, long, default_value_t = 255)]
    box_level: usize,
}

#[derive(Debug)]
struct ElementInfo {
    id: String,
    datatype: String,
    min: String,
    max: String,
}

#[derive(Debug)]
struct DocInfo {
    id: String,
    // value: Value,
    elements: Vec<ElementInfo>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::PlantUml(args) => {
            let docs = load_structure_definition_files(&args.common.files)?;
            let output = File::create(args.output_file)?;
            let mut writer = BufWriter::new(output); // Create a buffered writer

            writeln!(
                writer,
                "@startuml\nskinparam linetype polyline\nhide circle\nhide stereotype\nhide methods\n"
            )?;

            for doc in docs.iter() {
                println!("processing: {}", doc.id);
                writeln!(writer, "class **{}** {{", doc.id)?;
                let mut relations = Vec::<(String, String, String, String)>::new();

                for element in doc.elements.iter() {
                    if let Some(element_part) = get_slice_after_last_occurrence(&element.id, '.') {
                        let hier_level = count_char_occurrences(&element.id, '.') * 2;
                        // extract datatype doc.
                        let mut datatype = element.datatype.clone();
                        if datatype.starts_with("http") {
                            match get_slice_after_last_occurrence(&datatype, '/') {
                                Some(dt) => {
                                    datatype = dt;
                                }
                                None => {}
                            };
                        }

                        // if the datatype is one of the classes drawn, add a relation instead of a class element
                        if let Some(_) = docs.iter().position(|s| s.id == datatype) {
                            relations.push((
                                element_part,
                                datatype,
                                element.min.clone(),
                                element.max.clone(),
                            ));
                        } else {
                            if !args.common.elements_hide {
                                write!(
                                    writer,
                                    "{:>hier_level$}|_ {} : {}",
                                    "", element_part, datatype
                                )?;
                                if !args.common.cardinality_hide {
                                    write!(writer, " [{}..{}]", element.min, element.max)?;
                                }
                                writeln!(writer)?;
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
        Commands::Mindmap(args) => {
            let docs = load_structure_definition_files(&args.common.files)?;
            for doc in docs.iter() {
                println!("processing: {}", doc.id);
                let output = File::create(format!("{}.plantuml", doc.id))?;
                let mut writer = BufWriter::new(output); // Create a buffered writer

                writeln!(writer, "@startmindmap")?;

                writeln!(writer, "* {}", doc.id)?;

                for element in doc.elements.iter() {
                    if let Some(element_part) = get_slice_after_last_occurrence(&element.id, '.') {
                        let hier_level = count_char_occurrences(&element.id, '.') + 1;
                        writeln!(writer, 
                            "{}{} {}", 
                            "*".repeat(hier_level), 
                            if hier_level > args.box_level { "_" } else { "" },
                            element_part)?;
                    }
                }

                writeln!(writer, "@endmindmap")?;
            }
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

fn get_slice_after_last_occurrence(s: &String, c: char) -> Option<String> {
    if let Some(last_index) = s.rfind(c) {
        Some(s[last_index + c.len_utf8()..].to_string())
    } else {
        None
    }
}

fn count_char_occurrences(s: &String, c: char) -> usize {
    s.chars().filter(|&ch| ch == c).count()
}

fn load_structure_definition_files(
    files: &Vec<String>,
) -> Result<Vec<DocInfo>, Box<dyn std::error::Error>> {
    let mut docs = Vec::<DocInfo>::new();
    for file in files.iter() {
        match load_single_structure_definition_file(file) {
            Ok(doc_info) => {
                docs.push(doc_info);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
    Ok(docs)
}

fn load_single_structure_definition_file(
    file: &String,
) -> Result<DocInfo, Box<dyn std::error::Error>> {
    let doc = load_json_from_file(file)?;
    let id = doc["id"].as_str().ok_or("Missing id")?;
    let snapshot = doc["snapshot"]["element"]
        .as_array()
        .ok_or("Missing snapshot")?;
    let mut elements = Vec::<ElementInfo>::new();
    for element in snapshot.iter() {
        let element_id = element["id"].as_str().ok_or("Missing element id")?;
        if element_id != id {
            let datatype = element["type"][0]["code"]
                .as_str()
                .ok_or("Missing datatype")?;
            let min = if element["min"].is_string() {
                element["min"]
                    .as_str()
                    .ok_or(format!("Missing min cardinality: {:?}", element["min"]))?
                    .to_string()
            } else {
                element["min"].to_string()
            };
            let max = element["max"].as_str().ok_or("Missing max cardinality")?;
            elements.push(ElementInfo {
                id: element_id.to_string(),
                datatype: datatype.to_string(),
                min,
                max: max.to_string(),
            });
        }
    }
    Ok(DocInfo {
        id: id.to_string(),
        elements,
    })
}
