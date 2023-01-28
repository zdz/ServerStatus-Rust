use anyhow::Result;
use chrono::{Datelike, Local};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::str;

use crate::Args;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    #[serde(default = "Default::default")]
    pub day: u32,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RT {
    pub rx: u64,
    pub tx: u64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DateRT {
    pub id: u64,
    pub date: Date,
    pub rx: u64,
    pub tx: u64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Traffic {
    // pub id: u64,
    pub total: RT,
    // v1 months
    #[serde(default = "Default::default")]
    pub months: Vec<DateRT>,
    // v1 day
    #[serde(default = "Default::default")]
    pub days: Vec<DateRT>,
    // v2 month
    #[serde(default = "Default::default")]
    pub month: Vec<DateRT>,
    // v2 day
    #[serde(default = "Default::default")]
    pub day: Vec<DateRT>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Iface {
    // v1
    #[serde(default = "Default::default")]
    pub r#id: String,
    #[serde(default = "Default::default")]
    pub nick: String,
    // v2
    #[serde(default = "Default::default")]
    pub name: String,
    #[serde(default = "Default::default")]
    pub alias: String,
    // common
    pub traffic: Traffic,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct VnstatJson {
    pub vnstatversion: String,
    pub jsonversion: String,
    #[serde(default = "Default::default")]
    pub interfaces: Vec<Iface>,
}

fn calc_traffic(j: VnstatJson, mr: bool, args: &Args) -> Result<(u64, u64, u64, u64)> {
    let mut v1 = false;
    if j.jsonversion.eq("1") {
        v1 = true;
    } else if !j.jsonversion.eq("2") {
        panic!("vnstat version number must be 1 or 2");
    }

    let local_now = Local::now();
    let cur_year = local_now.year();
    let cur_month = local_now.month();
    let cur_day = local_now.day();
    let (mut network_in, mut network_out, mut m_network_in, mut m_network_out) = (0, 0, 0, 0);

    for iface in j.interfaces.iter() {
        let name = if v1 { &iface.r#id } else { &iface.name };
        if args.skip_iface(name) {
            continue;
        }

        network_in += iface.traffic.total.rx;
        network_out += iface.traffic.total.tx;

        if mr {
            // month rotate, v2 only
            if v1 {
                panic!("The parameter --json d 31 is not supported in v1.15");
            } else if cur_day >= args.vnstat_mr {
                for d in iface.traffic.day.iter() {
                    if d.date.year == cur_year && d.date.month == cur_month && d.date.day >= args.vnstat_mr {
                        m_network_in += d.rx;
                        m_network_out += d.tx;
                    }
                }
            } else {
                let mut pre_year = cur_year;
                let mut pre_month = cur_month - 1;
                if pre_month == 0 {
                    pre_month = 12;
                    pre_year -= 1;
                }

                for d in iface.traffic.day.iter() {
                    if d.date.year == pre_year && d.date.month == pre_month && d.date.day >= args.vnstat_mr {
                        m_network_in += d.rx;
                        m_network_out += d.tx;
                    }
                    if d.date.year == cur_year && d.date.month == cur_month && d.date.day < args.vnstat_mr {
                        m_network_in += d.rx;
                        m_network_out += d.tx;
                    }
                }
            }
        } else {
            // normal
            let month = if v1 {
                &iface.traffic.months
            } else {
                &iface.traffic.month
            };

            for m in month.iter() {
                if cur_year != m.date.year || cur_month != m.date.month {
                    continue;
                }
                m_network_in += m.rx;
                m_network_out += m.tx;
            }
        }
    }

    let factor: u64 = if v1 { 1024 } else { 1 };
    Ok((
        network_in * factor,
        network_out * factor,
        m_network_in * factor,
        m_network_out * factor,
    ))
}

pub fn get_traffic(args: &Args) -> Result<(u64, u64, u64, u64)> {
    if args.vnstat_mr == 1 {
        // !
        let a = Command::new("/usr/bin/vnstat")
            .args(["--json", "m"])
            .output()
            .expect("failed to execute vnstat")
            .stdout;
        let b = str::from_utf8(&a)?;
        let j: VnstatJson = serde_json::from_str(b).unwrap_or_else(|e| {
            error!("{:?}", e);
            panic!("invalid vnstat json `{b}")
        });
        calc_traffic(j, false, args)
    } else if args.vnstat_mr > 1 && args.vnstat_mr <= 28 {
        // month rotate
        let a = Command::new("/usr/bin/vnstat")
            .args(["--json", "d", "32"])
            .output()
            .expect("failed to execute vnstat")
            .stdout;
        let b = str::from_utf8(&a)?;
        let j: VnstatJson = serde_json::from_str(b).unwrap_or_else(|e| {
            error!("{:?}", e);
            panic!("invalid vnstat json `{b}")
        });
        calc_traffic(j, true, args)
    } else {
        panic!("invalid vnstat month rotate => `{}", args.vnstat_mr);
    }
}

#[allow(unused)]
#[cfg(test)]
mod tests {
    use crate::vnstat::VnstatJson;

    #[test]
    fn test_json_v1_m() {
        let json_v1: &str = r#"{"vnstatversion":"1.15","jsonversion":"1","interfaces":[{"id":"eth0","nick":"eth0","created":{"date":{"year":2022,"month":11,"day":8}},"updated":{"date":{"year":2022,"month":11,"day":8},"time":{"hour":17,"minutes":9}},"traffic":{"total":{"rx":376720,"tx":3780},"months":[{"id":0,"date":{"year":2022,"month":11},"rx":376720,"tx":3780}]}}]}"#;

        let j: VnstatJson = serde_json::from_str(json_v1).unwrap();
        dbg!(serde_json::to_string(&j).unwrap());

        assert!(j.jsonversion.eq("1"));
        assert!(!j.interfaces.is_empty());
        for iface in j.interfaces.iter() {
            assert!(!iface.r#id.is_empty());
            assert!(!iface.traffic.months.is_empty());
        }
    }

    #[test]
    fn test_json_v2_m() {
        let sl: Vec<&str> = vec![
            r#"{"vnstatversion":"2.6","jsonversion":"2","interfaces":[{"name":"ens3","alias":"","created":{"date":{"year":2022,"month":2,"day":13}},"updated":{"date":{"year":2022,"month":11,"day":8},"time":{"hour":17,"minute":15}},"traffic":{"total":{"rx":68731300786,"tx":93882257792},"month":[{"id":1,"date":{"year":2022,"month":2},"rx":21826588471,"tx":21311345467},{"id":2,"date":{"year":2022,"month":3},"rx":14368852952,"tx":13798123949},{"id":3,"date":{"year":2022,"month":4},"rx":7372736806,"tx":10448203738},{"id":4,"date":{"year":2022,"month":5},"rx":4743152375,"tx":8927493464},{"id":5,"date":{"year":2022,"month":6},"rx":3307523432,"tx":7557067196},{"id":6,"date":{"year":2022,"month":7},"rx":3454859957,"tx":8034854115},{"id":7,"date":{"year":2022,"month":8},"rx":3550404282,"tx":7409924068},{"id":8,"date":{"year":2022,"month":9},"rx":3756453283,"tx":6838553762},{"id":9,"date":{"year":2022,"month":10},"rx":4169681824,"tx":7177043572},{"id":10,"date":{"year":2022,"month":11},"rx":2181047404,"tx":2379648461}]}}]}"#,
            r#"{"vnstatversion":"2.9","jsonversion":"2","interfaces":[{"name":"eth0","alias":"","created":{"date":{"year":2022,"month":1,"day":26}},"updated":{"date":{"year":2022,"month":11,"day":9},"time":{"hour":1,"minute":10}},"traffic":{"total":{"rx":2192363238140,"tx":2052041474498},"month":[{"id":3,"date":{"year":2022,"month":1},"rx":12434171377,"tx":12307131934},{"id":7,"date":{"year":2022,"month":2},"rx":76020476445,"tx":76783617169},{"id":11,"date":{"year":2022,"month":3},"rx":165961951151,"tx":173062378510},{"id":15,"date":{"year":2022,"month":4},"rx":190149303832,"tx":206630170704},{"id":19,"date":{"year":2022,"month":5},"rx":185443064724,"tx":210610602138},{"id":23,"date":{"year":2022,"month":6},"rx":254414209251,"tx":279585149891},{"id":26,"date":{"year":2022,"month":7},"rx":584851983809,"tx":314275757844},{"id":29,"date":{"year":2022,"month":8},"rx":293027405965,"tx":314325067457},{"id":32,"date":{"year":2022,"month":9},"rx":384560035785,"tx":406968826006},{"id":35,"date":{"year":2022,"month":10},"rx":38991886948,"tx":45623593944},{"id":38,"date":{"year":2022,"month":11},"rx":6508748853,"tx":11869178901}]}},{"name":"eth1","alias":"","created":{"date":{"year":2022,"month":1,"day":26}},"updated":{"date":{"year":2022,"month":11,"day":9},"time":{"hour":1,"minute":10}},"traffic":{"total":{"rx":3264,"tx":248603366},"month":[{"id":2,"date":{"year":2022,"month":1},"rx":0,"tx":8531240},{"id":6,"date":{"year":2022,"month":2},"rx":0,"tx":47909412},{"id":10,"date":{"year":2022,"month":3},"rx":0,"tx":53099166},{"id":14,"date":{"year":2022,"month":4},"rx":420,"tx":51282336},{"id":18,"date":{"year":2022,"month":5},"rx":2352,"tx":52983378},{"id":22,"date":{"year":2022,"month":6},"rx":252,"tx":34584754},{"id":25,"date":{"year":2022,"month":7},"rx":240,"tx":50260},{"id":28,"date":{"year":2022,"month":8},"rx":0,"tx":50400},{"id":31,"date":{"year":2022,"month":9},"rx":0,"tx":48930},{"id":34,"date":{"year":2022,"month":10},"rx":0,"tx":50400},{"id":37,"date":{"year":2022,"month":11},"rx":0,"tx":13090}]}},{"name":"nebula","alias":"","created":{"date":{"year":2022,"month":1,"day":26}},"updated":{"date":{"year":2022,"month":11,"day":9},"time":{"hour":1,"minute":10}},"traffic":{"total":{"rx":71516373823,"tx":228930419137},"month":[{"id":1,"date":{"year":2022,"month":1},"rx":429628489,"tx":958972112},{"id":5,"date":{"year":2022,"month":2},"rx":3794288059,"tx":11127431856},{"id":9,"date":{"year":2022,"month":3},"rx":4847922889,"tx":16688012255},{"id":13,"date":{"year":2022,"month":4},"rx":6299107906,"tx":20554306509},{"id":17,"date":{"year":2022,"month":5},"rx":9671603263,"tx":30444801612},{"id":21,"date":{"year":2022,"month":6},"rx":10523493308,"tx":33739573071},{"id":24,"date":{"year":2022,"month":7},"rx":10973378979,"tx":35607126309},{"id":27,"date":{"year":2022,"month":8},"rx":10989247676,"tx":35489056098},{"id":30,"date":{"year":2022,"month":9},"rx":11285243824,"tx":34695731226},{"id":33,"date":{"year":2022,"month":10},"rx":2402427247,"tx":8408544251},{"id":36,"date":{"year":2022,"month":11},"rx":300032183,"tx":1216863838}]}}]}"#,
        ];

        for json_v2 in sl.iter() {
            let j: VnstatJson = serde_json::from_str(json_v2).unwrap();
            dbg!(serde_json::to_string(&j).unwrap());

            assert!(j.jsonversion.eq("2"));
            assert!(!j.interfaces.is_empty());
            for iface in j.interfaces.iter() {
                assert!(!iface.name.is_empty());
                assert!(!iface.traffic.month.is_empty());
            }
        }
    }

    #[test]
    fn test_json_v1_d31() {
        // v1.15 版本不支持参数 --json d 31
        assert!(true);
    }

    #[test]
    fn test_json_v2_d31() {
        let sl: Vec<&str> = vec![
            r#" {"vnstatversion":"2.6","jsonversion":"2","interfaces":[{"name":"eth0","alias":"","created":{"date":{"year":2022,"month":9,"day":7}},"updated":{"date":{"year":2022,"month":11,"day":8},"time":{"hour":0,"minute":40}},"traffic":{"total":{"rx":17839917859,"tx":9814377824},"day":[{"id":33,"date":{"year":2022,"month":10,"day":9},"rx":258783610,"tx":158309915},{"id":34,"date":{"year":2022,"month":10,"day":10},"rx":257131512,"tx":159859445},{"id":35,"date":{"year":2022,"month":10,"day":11},"rx":262121251,"tx":160969487},{"id":36,"date":{"year":2022,"month":10,"day":12},"rx":266443404,"tx":161454595},{"id":37,"date":{"year":2022,"month":10,"day":13},"rx":504634376,"tx":166494340},{"id":38,"date":{"year":2022,"month":10,"day":14},"rx":261502648,"tx":159184296},{"id":39,"date":{"year":2022,"month":10,"day":15},"rx":267634442,"tx":158500018},{"id":40,"date":{"year":2022,"month":10,"day":16},"rx":260921046,"tx":160741637},{"id":41,"date":{"year":2022,"month":10,"day":17},"rx":259567969,"tx":159025890},{"id":42,"date":{"year":2022,"month":10,"day":18},"rx":258021645,"tx":157247747},{"id":43,"date":{"year":2022,"month":10,"day":19},"rx":264941037,"tx":157837619},{"id":44,"date":{"year":2022,"month":10,"day":20},"rx":265873380,"tx":156460568},{"id":45,"date":{"year":2022,"month":10,"day":21},"rx":263277519,"tx":159796263},{"id":46,"date":{"year":2022,"month":10,"day":22},"rx":311951157,"tx":231372281},{"id":47,"date":{"year":2022,"month":10,"day":23},"rx":1457524610,"tx":248608543},{"id":48,"date":{"year":2022,"month":10,"day":24},"rx":255801181,"tx":159524445},{"id":49,"date":{"year":2022,"month":10,"day":25},"rx":257946594,"tx":158384179},{"id":50,"date":{"year":2022,"month":10,"day":26},"rx":269536310,"tx":158957974},{"id":51,"date":{"year":2022,"month":10,"day":27},"rx":352680356,"tx":166442244},{"id":52,"date":{"year":2022,"month":10,"day":28},"rx":266880865,"tx":157730877},{"id":53,"date":{"year":2022,"month":10,"day":29},"rx":263141738,"tx":158610479},{"id":54,"date":{"year":2022,"month":10,"day":30},"rx":260368022,"tx":159581645},{"id":55,"date":{"year":2022,"month":10,"day":31},"rx":258471897,"tx":158471158},{"id":56,"date":{"year":2022,"month":11,"day":1},"rx":259254834,"tx":158820245},{"id":57,"date":{"year":2022,"month":11,"day":2},"rx":477471458,"tx":167973778},{"id":58,"date":{"year":2022,"month":11,"day":3},"rx":297189929,"tx":165279212},{"id":59,"date":{"year":2022,"month":11,"day":4},"rx":461745307,"tx":167249546},{"id":60,"date":{"year":2022,"month":11,"day":5},"rx":262397032,"tx":161079967},{"id":61,"date":{"year":2022,"month":11,"day":6},"rx":303970015,"tx":205539194},{"id":62,"date":{"year":2022,"month":11,"day":7},"rx":260589211,"tx":162040277},{"id":63,"date":{"year":2022,"month":11,"day":8},"rx":7173232,"tx":4553451}]}}]} "#,
            r#"{"vnstatversion":"2.9","jsonversion":"2","interfaces":[{"name":"eth0","alias":"","created":{"date":{"year":2022,"month":1,"day":26}},"updated":{"date":{"year":2022,"month":11,"day":9},"time":{"hour":1,"minute":15}},"traffic":{"total":{"rx":2192371638731,"tx":2052047802937},"day":[{"id":920,"date":{"year":2022,"month":10,"day":10},"rx":1263089515,"tx":1380471538},{"id":923,"date":{"year":2022,"month":10,"day":11},"rx":459653734,"tx":622324659},{"id":926,"date":{"year":2022,"month":10,"day":12},"rx":785966270,"tx":947866873},{"id":929,"date":{"year":2022,"month":10,"day":13},"rx":294448526,"tx":449364458},{"id":932,"date":{"year":2022,"month":10,"day":14},"rx":286043964,"tx":450954310},{"id":935,"date":{"year":2022,"month":10,"day":15},"rx":314475587,"tx":456752018},{"id":938,"date":{"year":2022,"month":10,"day":16},"rx":456591140,"tx":494236831},{"id":941,"date":{"year":2022,"month":10,"day":17},"rx":265329185,"tx":455468592},{"id":944,"date":{"year":2022,"month":10,"day":18},"rx":279257008,"tx":459658581},{"id":947,"date":{"year":2022,"month":10,"day":19},"rx":287805724,"tx":449124847},{"id":950,"date":{"year":2022,"month":10,"day":20},"rx":267708493,"tx":459472253},{"id":953,"date":{"year":2022,"month":10,"day":21},"rx":295118358,"tx":453338036},{"id":956,"date":{"year":2022,"month":10,"day":22},"rx":298257131,"tx":452332397},{"id":959,"date":{"year":2022,"month":10,"day":23},"rx":428167662,"tx":558747170},{"id":962,"date":{"year":2022,"month":10,"day":24},"rx":272848993,"tx":458586017},{"id":965,"date":{"year":2022,"month":10,"day":25},"rx":598477854,"tx":895923344},{"id":968,"date":{"year":2022,"month":10,"day":26},"rx":546944032,"tx":700889855},{"id":971,"date":{"year":2022,"month":10,"day":27},"rx":293519746,"tx":452245489},{"id":974,"date":{"year":2022,"month":10,"day":28},"rx":2966477544,"tx":3167262056},{"id":977,"date":{"year":2022,"month":10,"day":29},"rx":4933756962,"tx":4907952838},{"id":980,"date":{"year":2022,"month":10,"day":30},"rx":3947128802,"tx":4050143988},{"id":983,"date":{"year":2022,"month":10,"day":31},"rx":432412074,"tx":643899886},{"id":986,"date":{"year":2022,"month":11,"day":1},"rx":450888572,"tx":648110313},{"id":989,"date":{"year":2022,"month":11,"day":2},"rx":3002279084,"tx":3188821232},{"id":992,"date":{"year":2022,"month":11,"day":3},"rx":477071540,"tx":678880479},{"id":995,"date":{"year":2022,"month":11,"day":4},"rx":440925290,"tx":633454401},{"id":998,"date":{"year":2022,"month":11,"day":5},"rx":406449162,"tx":637438337},{"id":1001,"date":{"year":2022,"month":11,"day":6},"rx":728338188,"tx":4404853477},{"id":1004,"date":{"year":2022,"month":11,"day":7},"rx":425151436,"tx":798135111},{"id":1007,"date":{"year":2022,"month":11,"day":8},"rx":463260366,"tx":789831334},{"id":1010,"date":{"year":2022,"month":11,"day":9},"rx":122785806,"tx":95982656}]}},{"name":"eth1","alias":"","created":{"date":{"year":2022,"month":1,"day":26}},"updated":{"date":{"year":2022,"month":11,"day":9},"time":{"hour":1,"minute":15}},"traffic":{"total":{"rx":3264,"tx":248603366},"day":[{"id":919,"date":{"year":2022,"month":10,"day":10},"rx":0,"tx":1610},{"id":922,"date":{"year":2022,"month":10,"day":11},"rx":0,"tx":1610},{"id":925,"date":{"year":2022,"month":10,"day":12},"rx":0,"tx":1680},{"id":928,"date":{"year":2022,"month":10,"day":13},"rx":0,"tx":1610},{"id":931,"date":{"year":2022,"month":10,"day":14},"rx":0,"tx":1680},{"id":934,"date":{"year":2022,"month":10,"day":15},"rx":0,"tx":1610},{"id":937,"date":{"year":2022,"month":10,"day":16},"rx":0,"tx":1610},{"id":940,"date":{"year":2022,"month":10,"day":17},"rx":0,"tx":1610},{"id":943,"date":{"year":2022,"month":10,"day":18},"rx":0,"tx":1610},{"id":946,"date":{"year":2022,"month":10,"day":19},"rx":0,"tx":1680},{"id":949,"date":{"year":2022,"month":10,"day":20},"rx":0,"tx":1610},{"id":952,"date":{"year":2022,"month":10,"day":21},"rx":0,"tx":1610},{"id":955,"date":{"year":2022,"month":10,"day":22},"rx":0,"tx":1680},{"id":958,"date":{"year":2022,"month":10,"day":23},"rx":0,"tx":1610},{"id":961,"date":{"year":2022,"month":10,"day":24},"rx":0,"tx":1610},{"id":964,"date":{"year":2022,"month":10,"day":25},"rx":0,"tx":1680},{"id":967,"date":{"year":2022,"month":10,"day":26},"rx":0,"tx":1610},{"id":970,"date":{"year":2022,"month":10,"day":27},"rx":0,"tx":1610},{"id":973,"date":{"year":2022,"month":10,"day":28},"rx":0,"tx":1610},{"id":976,"date":{"year":2022,"month":10,"day":29},"rx":0,"tx":1610},{"id":979,"date":{"year":2022,"month":10,"day":30},"rx":0,"tx":1610},{"id":982,"date":{"year":2022,"month":10,"day":31},"rx":0,"tx":1610},{"id":985,"date":{"year":2022,"month":11,"day":1},"rx":0,"tx":1680},{"id":988,"date":{"year":2022,"month":11,"day":2},"rx":0,"tx":1610},{"id":991,"date":{"year":2022,"month":11,"day":3},"rx":0,"tx":1610},{"id":994,"date":{"year":2022,"month":11,"day":4},"rx":0,"tx":1610},{"id":997,"date":{"year":2022,"month":11,"day":5},"rx":0,"tx":1610},{"id":1000,"date":{"year":2022,"month":11,"day":6},"rx":0,"tx":1610},{"id":1003,"date":{"year":2022,"month":11,"day":7},"rx":0,"tx":1610},{"id":1006,"date":{"year":2022,"month":11,"day":8},"rx":0,"tx":1610},{"id":1009,"date":{"year":2022,"month":11,"day":9},"rx":0,"tx":140}]}},{"name":"nebula","alias":"","created":{"date":{"year":2022,"month":1,"day":26}},"updated":{"date":{"year":2022,"month":11,"day":9},"time":{"hour":1,"minute":15}},"traffic":{"total":{"rx":71516717099,"tx":228930536555},"day":[{"id":918,"date":{"year":2022,"month":10,"day":10},"rx":30760856,"tx":145633627},{"id":921,"date":{"year":2022,"month":10,"day":11},"rx":30809642,"tx":146290097},{"id":924,"date":{"year":2022,"month":10,"day":12},"rx":30669563,"tx":145540361},{"id":927,"date":{"year":2022,"month":10,"day":13},"rx":31855235,"tx":146589445},{"id":930,"date":{"year":2022,"month":10,"day":14},"rx":38883889,"tx":146348470},{"id":933,"date":{"year":2022,"month":10,"day":15},"rx":44844949,"tx":147077059},{"id":936,"date":{"year":2022,"month":10,"day":16},"rx":52495473,"tx":168732677},{"id":939,"date":{"year":2022,"month":10,"day":17},"rx":41190293,"tx":147176867},{"id":942,"date":{"year":2022,"month":10,"day":18},"rx":35659811,"tx":149568925},{"id":945,"date":{"year":2022,"month":10,"day":19},"rx":31136493,"tx":146624931},{"id":948,"date":{"year":2022,"month":10,"day":20},"rx":40557985,"tx":148250527},{"id":951,"date":{"year":2022,"month":10,"day":21},"rx":33576138,"tx":146742687},{"id":954,"date":{"year":2022,"month":10,"day":22},"rx":33586766,"tx":146605923},{"id":957,"date":{"year":2022,"month":10,"day":23},"rx":43805188,"tx":176152003},{"id":960,"date":{"year":2022,"month":10,"day":24},"rx":45049416,"tx":147173472},{"id":963,"date":{"year":2022,"month":10,"day":25},"rx":47504459,"tx":228585565},{"id":966,"date":{"year":2022,"month":10,"day":26},"rx":72275564,"tx":152033235},{"id":969,"date":{"year":2022,"month":10,"day":27},"rx":32244322,"tx":147942336},{"id":972,"date":{"year":2022,"month":10,"day":28},"rx":30490277,"tx":144065990},{"id":975,"date":{"year":2022,"month":10,"day":29},"rx":30333864,"tx":142967094},{"id":978,"date":{"year":2022,"month":10,"day":30},"rx":45573303,"tx":181850898},{"id":981,"date":{"year":2022,"month":10,"day":31},"rx":39601604,"tx":150652615},{"id":984,"date":{"year":2022,"month":11,"day":1},"rx":35559909,"tx":143545626},{"id":987,"date":{"year":2022,"month":11,"day":2},"rx":32050806,"tx":143669861},{"id":990,"date":{"year":2022,"month":11,"day":3},"rx":45674634,"tx":150791733},{"id":993,"date":{"year":2022,"month":11,"day":4},"rx":35807721,"tx":149407573},{"id":996,"date":{"year":2022,"month":11,"day":5},"rx":35975767,"tx":149653083},{"id":999,"date":{"year":2022,"month":11,"day":6},"rx":52570317,"tx":203214295},{"id":1002,"date":{"year":2022,"month":11,"day":7},"rx":30480300,"tx":141737647},{"id":1005,"date":{"year":2022,"month":11,"day":8},"rx":28112622,"tx":133564436},{"id":1008,"date":{"year":2022,"month":11,"day":9},"rx":4143383,"tx":1397002}]}}]}"#,
        ];
        for json_v2_d31 in sl.iter() {
            let j: VnstatJson = serde_json::from_str(json_v2_d31).unwrap();
            dbg!(serde_json::to_string(&j).unwrap());

            assert!(j.jsonversion.eq("2"));
            assert!(!j.interfaces.is_empty());
            for iface in j.interfaces.iter() {
                assert!(!iface.name.is_empty());
                assert!(!iface.traffic.day.is_empty());
            }
        }
    }
}
