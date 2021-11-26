mod trains {
    use suds_macro::suds;
    suds! {"https://lite.realtime.nationalrail.co.uk/OpenLDBWS/wsdl.aspx?ver=2017-10-01"}
}

fn main() {
    let message = trains::messages::GetNextDeparturesWithDetailsSoapIn {
        parameters: trains::types::GetNextDeparturesWithDetailsRequest {
            crs: trains::types::CRSType("BSK".into()),
            filterList: trains::types::CRSType("BHM".into()),
            timeOffset: 0,
            timeWindow: 120
        }
    };

    let envelope = suds_util::soap::Envelope::new(message);
    println!("{}", String::from_utf8(envelope.to_request()).unwrap());
}
