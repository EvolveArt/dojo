use std::collections::HashMap;
use std::iter::zip;
use std::ops::DerefMut;

use anyhow::{Context, Result};
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_filesystem::db::FilesGroup;
use cairo_lang_filesystem::ids::CrateLongId;
use cairo_lang_starknet::casm_contract_class::CasmContractClass;
use cairo_lang_starknet::contract::find_contracts;
use cairo_lang_starknet::contract_class::{compile_prepared_db, ContractClass};
use cairo_lang_utils::Upcast;
use scarb::compiler::helpers::{
    build_compiler_config, build_project_config, collect_main_crate_ids,
};
use scarb::compiler::{CompilationUnit, Compiler};
use scarb::core::Workspace;
use smol_str::SmolStr;
use starknet::core::types::contract::CompiledClass;
use starknet::core::types::FieldElement;
use tracing::{trace, trace_span};

use crate::db::DojoRootDatabaseBuilderEx;
use crate::manifest::Manifest;

#[cfg(test)]
#[path = "compiler_test.rs"]
mod test;

pub struct DojoCompiler;

impl Compiler for DojoCompiler {
    fn target_kind(&self) -> &str {
        "dojo"
    }

    fn compile(&self, unit: CompilationUnit, ws: &Workspace<'_>) -> Result<()> {
        let target_dir = unit.profile.target_dir(ws.config());

        let mut db = RootDatabase::builder()
            .with_project_config(build_project_config(&unit)?)
            .with_dojo()
            .build()?;

        let compiler_config = build_compiler_config(&unit, ws);

        let mut main_crate_ids = collect_main_crate_ids(&unit, &db);
        if unit.main_component().cairo_package_name() != "dojo_core" {
            main_crate_ids.push(db.intern_crate(CrateLongId("dojo_core".into())));
        }

        let contracts = {
            let _ = trace_span!("find_contracts").enter();
            find_contracts(&db, &main_crate_ids)
        };

        trace!(
            contracts = ?contracts
                .iter()
                .map(|decl| decl.module_id().full_path(db.upcast()))
                .collect::<Vec<_>>()
        );

        let contracts = contracts.iter().collect::<Vec<_>>();

        let classes = {
            let _ = trace_span!("compile_starknet").enter();
            compile_prepared_db(&mut db, &contracts, compiler_config)?
        };

        // (contract name, class hash)
        let mut compiled_classes: HashMap<SmolStr, FieldElement> = HashMap::new();

        for (decl, class) in zip(contracts, classes) {
            let target_name = &unit.target().name;
            let contract_name = decl.submodule_id.name(db.upcast());
            let file_name = format!("{target_name}_{contract_name}.json");

            let mut file = target_dir.open_rw(file_name.clone(), "output file", ws.config())?;
            serde_json::to_writer_pretty(file.deref_mut(), &class)
                .with_context(|| format!("failed to serialize contract: {contract_name}"))?;

            let class_hash = compute_class_hash_of_contract_class(&contract_name, class)?;
            compiled_classes.insert(contract_name, class_hash);
        }

        let mut file = target_dir.open_rw("manifest.json", "output file", ws.config())?;
        let manifest = Manifest::new(&db, &main_crate_ids, compiled_classes);
        serde_json::to_writer_pretty(file.deref_mut(), &manifest)
            .with_context(|| "failed to serialize manifest")?;

        Ok(())
    }
}

fn compute_class_hash_of_contract_class(
    contract_name: &str,
    class: ContractClass,
) -> Result<FieldElement> {
    let casm_contract = CasmContractClass::from_contract_class(class, true)
        .with_context(|| "Compilation failed.")?;
    let class_json = serde_json::to_string_pretty(&casm_contract)
        .with_context(|| "Casm contract Serialization failed.")?;
    let compiled_class: CompiledClass = serde_json::from_str(&class_json).unwrap_or_else(|error| {
        panic!("Problem parsing {contract_name} artifact: {error:?}");
    });

    compiled_class.class_hash().with_context(|| "Casm contract Serialization failed.")
}