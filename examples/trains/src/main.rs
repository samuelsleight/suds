mod trains {
    use suds_macro::suds;
    suds! {"https://lite.realtime.nationalrail.co.uk/OpenLDBWS/wsdl.aspx?ver=2017-10-01"}
}

fn main() {
    let message = trains::messages::GetNextDeparturesWithDetailsSoapIn {
        parameters: trains::types::GetNextDeparturesWithDetailsRequest {
            crs: trains::types::CRSType("BSK"),
            filterList: trains::types::CRSType("BHM"),
            timeOffset: 0,
            timeWindow: 120
        }
    };

    let envelope = suds_util::soap::Envelope::new(message);
    println!("{}", envelope.into_body());
}
