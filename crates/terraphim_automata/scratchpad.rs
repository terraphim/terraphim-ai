use std::collections::HashSet;

pub fn get_edges(
    settings: &Settings,
    nodes: &[String],
    years: Option<&[i64]>,
    limits: i64,
    mnodes: &std::collections::HashSet<String>,
) -> Vec<Edge> {
    let mut links = Vec::new();
    let mut edges_set = HashSet::new();
    let url = settings.redis_url.clone();
    let client = redis::Client::open(url).unwrap();
    let mut conn = client.get_connection().unwrap();
    let graph_name = "cord19medical";
    let query = if let Some(years) = years {
        format!(
            "WITH $ids as ids
            MATCH (e:entity)-[r]->(t:entity)
            WHERE e.id IN ids AND r.year IN $years
            RETURN DISTINCT e.id, t.id, max(r.rank), r.year
            ORDER BY r.rank DESC LIMIT {}",
            limits
        )
    } else {
        format!(
            "WITH $ids as ids
            MATCH (e:entity)-[r]->(t:entity)
            WHERE e.id IN ids
            RETURN DISTINCT e.id, t.id, max(r.rank), r.year
            ORDER BY r.rank DESC LIMIT {}",
            limits
        )
    };
    let params = if let Some(years) = years {
        redis::cmd("WITH $years as years RETURN years")
            .arg(years)
            .query(&mut conn)
            .unwrap()
    } else {
        redis::cmd("MATCH (e:entity) RETURN DISTINCT e.year")
            .query(&mut conn)
            .unwrap()
    };
    let years: Vec<i64> = params[0]
        .as_array()
        .unwrap()
        .iter()
        .map(|year| year.as_i64().unwrap())
        .collect();
    let ids: Vec<&str> = nodes.iter().map(|node| node.as_str()).collect();
    let params = redis::cmd(query)
        .arg(redis::json::to_value(&ids).unwrap())
        .arg(redis::json::to_value(&years).unwrap())
        .query(&mut conn)
        .unwrap();
    for row in params.as_array().unwrap() {
        let id1 = row[0].as_str().unwrap().to_string();
        let id2 = row[1].as_str().unwrap().to_string();
        let rank = row[2].as_i64().unwrap();
        let year = row[3].as_i64().unwrap();
        let edge = Edge {
            source_id: id1.clone(),
            target_id: id2.clone(),
            rank: rank as u64,
            year: Some(year),
        };
        if edges_set.insert((id1, id2)) {
            links.push(edge);
        }
    }
    links
}
// let client = reqwest::Client::new();
// let res = client
//     .get("https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json")
//     .header(CONTENT_TYPE, "application/json")
//     .send()
//     .await
//     .unwrap()
//     .text()
//     .await
//     .unwrap();

// let deser_map: HashMap<String, NormalizedTerm> = json_to_map(&res).unwrap();
// println!("{:?}", deser_map);
// let mut thesaurus:  = HashMap::new();
// thesaurus = serde_json::from_str(&res).unwrap();
// let resp200 = res.json::<HashMap<String, NormalizedTerm>>().await?;
// let resp200 = client
//     .get(json_url)
//     .header(CONTENT_TYPE, "application/json")
//     .send()
//     .await?
//     .json::<DictHash>()
//     .await?;
