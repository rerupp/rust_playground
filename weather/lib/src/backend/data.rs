//! This module holds data shared between the file system and database.
#![allow(unused)]

const STATES_AND_IDS: [(&'static str, &'static str, &'static str, &'static str); 52] = [
    ("Alabama", "alabama", "AL", "al"),
    ("Alaska", "alaska", "AK", "ak"),
    ("Arizona", "arizona", "AZ", "az"),
    ("Arkansas", "arkansas", "AR", "ar"),
    ("California", "california", "CA", "ca"),
    ("Colorado", "colorado", "CO", "co"),
    ("Connecticut", "connecticut", "CT", "ct"),
    ("Delaware", "delaware", "DE", "de"),
    ("District of Columbia", "district of columbia", "DC", "dc"),
    ("Florida", "florida", "FL", "fl"),
    ("Georgia", "georgia", "GA", "ga"),
    ("Hawaii", "hawaii", "HI", "hi"),
    ("Idaho", "idaho", "ID", "id"),
    ("Illinois", "illinois", "IL", "il"),
    ("Indiana", "indiana", "IN", "in"),
    ("Iowa", "iowa", "IA", "ia"),
    ("Kansas", "kansas", "KS", "ks"),
    ("Kentucky", "kentucky", "KY", "ky"),
    ("Louisiana", "louisiana", "LA", "la"),
    ("Maine", "maine", "ME", "me"),
    ("Maryland", "maryland", "MD", "md"),
    ("Massachusetts", "massachusetts", "MA", "ma"),
    ("Michigan", "michigan", "MI", "mi"),
    ("Minnesota", "minnesota", "MN", "mn"),
    ("Mississippi", "mississippi", "MS", "ms"),
    ("Missouri", "missouri", "MO", "mo"),
    ("Montana", "montana", "MT", "mt"),
    ("Nebraska", "nebraska", "NE", "ne"),
    ("Nevada", "nevada", "NV", "nv"),
    ("New Hampshire", "new hampshire", "NH", "nh"),
    ("New Jersey", "new jersey", "NJ", "nj"),
    ("New Mexico", "new mexico", "NM", "nm"),
    ("New York", "new york", "NY", "ny"),
    ("North Carolina", "north carolina", "NC", "nc"),
    ("North Dakota", "north dakota", "ND", "nd"),
    ("Ohio", "ohio", "OH", "oh"),
    ("Oklahoma", "oklahoma", "OK", "ok"),
    ("Oregon", "oregon", "OR", "or"),
    ("Pennsylvania", "pennsylvania", "PA", "pa"),
    ("Puerto Rico", "puerto rico", "PR", "pr"),
    ("Rhode Island", "rhode island", "RI", "ri"),
    ("South Carolina", "south carolina", "SC", "sc"),
    ("South Dakota", "south dakota", "SD", "sd"),
    ("Tennessee", "tennessee", "TN", "tn"),
    ("Texas", "texas", "TX", "tx"),
    ("Utah", "utah", "UT", "ut"),
    ("Vermont", "vermont", "VT", "vt"),
    ("Virginia", "virginia", "VA", "va"),
    ("Washington", "washington", "WA", "wa"),
    ("West Virginia", "west virginia", "WV", "wv"),
    ("Wisconsin", "wisconsin", "WI", "wi"),
    ("Wyoming", "wyoming", "WY", "wy"),
];

pub fn get_state_id(state_id: &str) -> Option<&'static str> {
    let state_id = state_id.trim().to_ascii_lowercase();
    STATES_AND_IDS.iter().find_map(|row| match &state_id == row.3 {
        true => Some(row.2),
        false => None
    })
}

pub fn get_state(state: &str) -> Option<&'static str> {
    // remove whitespace from the state JIC
    let state = state.split_whitespace().collect::<Vec<&str>>().join(" ").to_ascii_lowercase();
    STATES_AND_IDS.iter().find_map(|row| match &state == row.1 {
        true => Some(row.0),
        false => None
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_get_state_id() {
        assert_eq!(get_state_id(""), None);
        assert_eq!(get_state_id("Va"), Some("VA"));
        assert_eq!(get_state_id("oR"), Some("OR"));
        assert_eq!(get_state_id("WY"), Some("WY"));
    }
    
    #[test]
    fn test_get_state() {
        assert_eq!(get_state("foo"), None);
        assert_eq!(get_state("New   york"), Some("New York"));
        assert_eq!(get_state("teXas"), Some("Texas"));
    }
}