use std::{collections::{HashMap, HashSet}, env, fs::File, path::Path};

use iri_s::IriS;
use oxrdf::{IriParseError, NamedNode, Term};
use prefixmap::{PrefixMap, PrefixMapError};
use serde::Serialize;
use srdf::{SRDFBasic, SRDFGraph, SRDFGraphError, SRDF};

#[derive(Serialize)]
struct Matches {
    matches: Vec<MatchItem>
}

#[derive(Serialize)]
struct MatchItem {
    trigger: String,
    replace: String,
    label: String,
}

fn find_subjects(graph: &SRDFGraph, pred: &<SRDFGraph as SRDFBasic>::IRI, object: &<SRDFGraph as SRDFBasic>::Term, label_type: &str, use_label_when_possible: bool) -> Result<Vec<MatchItem>, AppError> {
    let pm = PrefixMap::from_hashmap(&HashMap::from([
        ("Core", "https://spec.industrialontologies.org/ontology/core/Core/"),
        ("owl", "http://www.w3.org/2002/07/owl#"),
        ("bfo", "http://purl.obolibrary.org/obo/"),
    ]))?;
    let rdfs_label = NamedNode::new("http://www.w3.org/2000/01/rdf-schema#label")?;

    let mut items: Vec<MatchItem> = Vec::new();
    for subject in graph.subjects_with_predicate_object(pred, object)? {
        let labels = graph.objects_for_subject_predicate(&subject, &rdfs_label)?;
        let english_label = get_english_label(&labels);

        let subj_iri = match subject {
            oxrdf::Subject::NamedNode(named_node) => IriS::from_named_node(&named_node),
            _ => continue
        };
        let qualified_name = pm.qualify(&subj_iri);

        items.push(MatchItem {
            trigger: if use_label_when_possible && english_label.is_some() {
                format!(":{}", english_label.unwrap().replace(" ", "-"))
            } else {
                format!(":{}", qualified_name)
            },
            replace: if use_label_when_possible && english_label.is_some() {
                format!("{} ({})", english_label.unwrap(), qualified_name)
            } else {
                qualified_name.clone()
            },
            label: if use_label_when_possible && english_label.is_some() {
                format!("{} ({})", english_label.unwrap(), label_type)
            } else {
                format!("{} ({})", qualified_name, label_type)
            }
        });
    }
    Ok(items)
}

fn get_english_label(labels: &HashSet<Term>) -> Option<&str> {
    for label in labels {
        let literal_content  = match label {
            oxrdf::Term::Literal(l) => l,
            _ => return None
        };

        if let Some(lang) = literal_content.language() {
            if lang == "en" {
                return Some(literal_content.value());
            }
        }
    }
    None
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("usage: {} <FILE>", args.get(0).unwrap());
        std::process::exit(1);
    }

    let filename = match args.get(1) {
        Some(f) => f,
        None => std::process::exit(1),
    };
    let path = Path::new(filename);
    let graph = SRDFGraph::from_path(path, &srdf::RDFFormat::RDFXML, None, &srdf::ReaderMode::Lax).unwrap();
    
    println!("Graph's len: {}", graph.len());

    // for quad in graph.quads() {
    //     println!("{}", quad);
    // }

    let pm = PrefixMap::from_hashmap(&HashMap::from([
        ("Core", "https://spec.industrialontologies.org/ontology/core/Core/"),
        ("owl", "http://www.w3.org/2002/07/owl#"),
        ("bfo", "http://purl.obolibrary.org/obo/"),
    ])).unwrap();

    let owl_import = NamedNode::new("http://www.w3.org/2002/07/owl#imports").unwrap();
    let rdf_type = NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type").unwrap();
    let owl_class = NamedNode::new("http://www.w3.org/2002/07/owl#Class").unwrap();
    let owl_object_property = NamedNode::new("http://www.w3.org/2002/07/owl#ObjectProperty").unwrap();

    println!("\n\nImports:");
    for triple in graph.triples_with_predicate(&owl_import).unwrap() {
        println!("{}", triple);
    }

    let use_label_when_possible: bool = true;

    let mut items: Vec<MatchItem> = Vec::new();
    let result = find_subjects(&graph, &rdf_type, &Term::from(owl_class), "Class", use_label_when_possible);
    match result {
        Ok(mut class_items) => items.append(&mut class_items),
        Err(_) => panic!("failed to find subjects for Class")
    }

    let result = find_subjects(&graph, &rdf_type, &Term::from(owl_object_property), "Object Property", use_label_when_possible);
    match result {
        Ok(mut object_property_items) => items.append(&mut object_property_items),
        Err(_) => panic!("failed to find subjects for Object Property")
    }

    let out_filepath = Path::new("packages.yml");
    let out_file = match File::create(&out_filepath) {
        Err(why) => panic!("couldn't open {}: {}", out_filepath.display(), why),
        Ok(file) => file,
    };
    
    match serde_yml::to_writer(out_file, &Matches { matches: items }) {
        Err(why) => panic!("couldn't write YAML data: {}", why),
        Ok(_) => println!("Write completed."),
    }
}

enum AppError {
    AppError
}

impl From<PrefixMapError> for AppError {
    fn from(value: PrefixMapError) -> Self {
        AppError::AppError
    }
}

impl From<IriParseError> for AppError {
    fn from(value: IriParseError) -> Self {
        AppError::AppError
    }
}

impl From<SRDFGraphError> for AppError {
    fn from(value: SRDFGraphError) -> Self {
        AppError::AppError
    }
}