use std::{collections::HashMap, env, fs::File, path::Path};

use iri_s::IriS;
use oxrdf::{vocab::rdf, NamedNode, Term};
use prefixmap::PrefixMap;
use serde::Serialize;
use srdf::{SRDFGraph, SRDF};

#[derive(Serialize)]
struct Matches {
    matches: Vec<MatchItem>
}

#[derive(Serialize)]
struct MatchItem {
    trigger: String,
    replace: String,
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
        ("owl", "http://www.w3.org/2002/07/owl#")
    ])).unwrap();

    let owl_import = NamedNode::new("http://www.w3.org/2002/07/owl#imports").unwrap();
    let rdfs_type = NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type").unwrap();
    let owl_object_property = NamedNode::new("http://www.w3.org/2002/07/owl#ObjectProperty").unwrap();

    println!("\n\nImports:");
    for triple in graph.triples_with_predicate(&owl_import).unwrap() {
        println!("{}", triple);
    }

    let mut items: Vec<MatchItem> = Vec::new();

    println!("\n\nObject Properties:");
    for subject in graph.subjects_with_predicate_object(&rdfs_type, &Term::from(owl_object_property)).unwrap() {
        let subj_iri = match subject {
            oxrdf::Subject::NamedNode(named_node) => IriS::from_named_node(&named_node),
            _ => continue
        };
        items.push(MatchItem {
            trigger: format!(":{}", pm.qualify(&subj_iri)),
            replace: pm.qualify(&subj_iri)
        });
    }

    let out_filepath = Path::new("packages.yml");
    let mut out_file = match File::create(&out_filepath) {
        Err(why) => panic!("couldn't open {}: {}", out_filepath.display(), why),
        Ok(file) => file,
    };
    
    match serde_yml::to_writer(out_file, &Matches { matches: items }) {
        Err(why) => panic!("couldn't write YAML data: {}", why),
        Ok(_) => println!("Write completed."),
    }
}
