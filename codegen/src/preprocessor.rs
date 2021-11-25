use super::types;
use suds_wsdl::types as wsdl;

pub fn preprocess(definition: &wsdl::Definition) -> types::Definition {
    let mut services = Vec::new();

    for service in &definition.services {
        let mut ports = Vec::new();

        for port in &service.ports {
            let binding = if let Some(binding) = definition
                .bindings
                .iter()
                .find(|binding| binding.name == port.binding)
            {
                binding
            } else {
                unimplemented!()
            };

            let port_type = if let Some(port_type) = definition
                .port_types
                .iter()
                .find(|port_type| port_type.name == binding.ty)
            {
                port_type
            } else {
                unimplemented!()
            };

            ports.push(types::Port {
                name: port.name.clone(),
                location: port.location.clone(),
                operations: port_type.operations.clone(),
            });
        }

        services.push(types::Service {
            name: service.name.clone(),
            ports,
        });
    }

    types::Definition {
        services,
        messages: definition.messages.clone(),
        types: definition.types.clone(),
    }
}
