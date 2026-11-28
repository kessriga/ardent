use std::collections::HashSet;
use std::collections::btree_map::Entry;

use ardent_core::properties::{NodeProperties, Property};

use super::{CALL_FUEL, MAX_SOURCE_BYTES, Runtime, RuntimeError};

wasmtime::component::bindgen!({ path: "../../wit", world: "source-properties" });
use exports::ardent::format::node_properties::Group;

impl Runtime {
    pub(crate) fn properties(
        &mut self,
        source: &[u8],
        outline_len: usize,
    ) -> Result<NodeProperties, RuntimeError> {
        self.properties_with_fuel(source, outline_len, CALL_FUEL)
    }

    fn properties_with_fuel(
        &mut self,
        source: &[u8],
        outline_len: usize,
        fuel: u64,
    ) -> Result<NodeProperties, RuntimeError> {
        if source.len() > MAX_SOURCE_BYTES {
            return Err(RuntimeError::Invalid("source exceeds property input limit"));
        }
        let groups = self.call(|i| {
            if i.component
                .get_export_index(&mut i.store, None, "ardent:format/node-properties@0.1.0")
                .is_none()
            {
                return Ok(Vec::new());
            }
            let bindings = SourceProperties::new(&mut i.store, &i.component)?;
            i.prepare()?;
            i.store.set_fuel(fuel)?;
            bindings
                .ardent_format_node_properties()
                .call_properties(&mut i.store, source)
        })?;
        validate(groups, outline_len)
    }
}

fn validate(groups: Vec<Group>, outline_len: usize) -> Result<NodeProperties, RuntimeError> {
    let mut by_node = NodeProperties::new();
    for group in groups {
        let scanner_index = group.node as usize;
        if scanner_index >= outline_len.saturating_sub(1) {
            return Err(RuntimeError::Invalid("property node outside outline"));
        }
        let Entry::Vacant(entry) = by_node.entry(scanner_index + 1) else {
            return Err(RuntimeError::Invalid("duplicate property group"));
        };
        let mut names = HashSet::new();
        for property in &group.properties {
            if property.name.trim().is_empty() || !names.insert(property.name.as_str()) {
                return Err(RuntimeError::Invalid("empty or duplicate property name"));
            }
        }
        entry.insert(
            group
                .properties
                .into_iter()
                .map(|p| Property {
                    name: p.name,
                    value: p.value,
                    inline: p.inline,
                })
                .collect(),
        );
    }
    Ok(by_node)
}

#[cfg(test)]
mod tests {
    use super::*;
    use exports::ardent::format::node_properties::Property as GuestProperty;

    fn group(node: u32, properties: &[(&str, &str, bool)]) -> Group {
        Group {
            node,
            properties: properties
                .iter()
                .map(|&(name, value, inline)| GuestProperty {
                    name: name.into(),
                    value: value.into(),
                    inline,
                })
                .collect(),
        }
    }

    #[test]
    fn preserves_names_values_order_and_visibility_at_the_outline_index() -> Result<(), RuntimeError>
    {
        let expected = [("range", "0..5", false), ("checked", "true", true)];
        let values = validate(vec![group(1, &expected)], 3)?;
        assert_eq!(values.len(), 1);
        assert_eq!(
            values[&2]
                .iter()
                .map(|p| (p.name.as_str(), p.value.as_str(), p.inline))
                .collect::<Vec<_>>(),
            expected
        );
        Ok(())
    }

    #[test]
    fn rejects_the_whole_result_for_invalid_nodes_or_names() {
        let valid = || group(0, &[("x", "1", true)]);
        for invalid in [
            group(2, &[]),
            group(u32::MAX, &[]),
            valid(),
            group(1, &[("x", "1", true), ("x", "2", false)]),
            group(1, &[("", "1", false)]),
        ] {
            assert!(validate(vec![valid(), invalid], 3).is_err());
        }
    }

    #[test]
    fn recovers_after_a_property_trap() -> Result<(), Box<dyn std::error::Error>> {
        let bytes = include_bytes!("../../../../plugins/markdown/format.wasm");
        let mut runtime = Runtime::compile(bytes)?;
        let source = b"- [x] Done\n";
        assert!(runtime.properties_with_fuel(source, 3, 0).is_err());
        assert!(runtime.active.is_none());
        assert!(matches!(
            runtime.properties(source, 1),
            Err(RuntimeError::Invalid("property node outside outline"))
        ));
        assert_eq!(runtime.properties(source, 3)?[&2][0].name, "checked");
        assert_eq!(runtime.scan("test.md", source)?.len(), 1);
        Ok(())
    }
}
