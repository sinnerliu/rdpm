use crate::model::server::{ServerEntry, ServerGroup, ServerTree};

/// 搜索与过滤匹配器
pub struct ServerSearch;

impl ServerSearch {
    /// 检查单个服务器是否命中搜索关键词
    pub fn matches(server: &ServerEntry, query: &str) -> bool {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return true;
        }

        // 1. 匹配服务器名称（忽略大小写）
        if server.name.to_lowercase().contains(&q) {
            return true;
        }

        // 2. 匹配 IP 地址或主机名
        if server.host.to_lowercase().contains(&q) {
            return true;
        }

        // 3. 匹配用户名
        if server.username.to_lowercase().contains(&q) {
            return true;
        }

        // 4. 匹配备注信息
        if let Some(notes) = &server.notes {
            if notes.to_lowercase().contains(&q) {
                return true;
            }
        }

        // 5. 拼音首字母简易匹配
        let pinyin_initials = get_pinyin_initials(&server.name);
        if pinyin_initials.contains(&q) {
            return true;
        }

        false
    }

    /// 根据查询关键词过滤整棵服务器树，生成包含匹配结果的新树（自动展开匹配的分组）
    pub fn filter_tree(tree: &ServerTree, query: &str) -> ServerTree {
        let q = query.trim();
        if q.is_empty() {
            return tree.clone();
        }

        let filtered_ungrouped: Vec<ServerEntry> = tree
            .ungrouped_servers
            .iter()
            .filter(|s| Self::matches(s, q))
            .cloned()
            .collect();

        let mut filtered_groups = Vec::new();
        for group in &tree.groups {
            if let Some(filtered_g) = filter_group(group, q) {
                filtered_groups.push(filtered_g);
            }
        }

        ServerTree {
            groups: filtered_groups,
            ungrouped_servers: filtered_ungrouped,
        }
    }
}

fn filter_group(group: &ServerGroup, query: &str) -> Option<ServerGroup> {
    let mut matching_servers = Vec::new();
    for s in &group.servers {
        if ServerSearch::matches(s, query) {
            matching_servers.push(s.clone());
        }
    }

    let mut matching_subgroups = Vec::new();
    for sub in &group.subgroups {
        if let Some(filtered_sub) = filter_group(sub, query) {
            matching_subgroups.push(filtered_sub);
        }
    }

    // 若分组名称本身命中，或者其下包含命中的服务器或子分组，则保留该分组
    let group_name_matched = group.name.to_lowercase().contains(&query.to_lowercase());
    if group_name_matched || !matching_servers.is_empty() || !matching_subgroups.is_empty() {
        let mut new_group = group.clone();
        new_group.is_expanded = true; // 搜索时自动展开命中项
        new_group.servers = if group_name_matched && matching_servers.is_empty() {
            group.servers.clone()
        } else {
            matching_servers
        };
        new_group.subgroups = matching_subgroups;
        Some(new_group)
    } else {
        None
    }
}

/// 提取文本的简拼首字母（覆盖常见字及运维高频词）
fn get_pinyin_initials(text: &str) -> String {
    let mut initials = String::new();
    for c in text.chars() {
        if c.is_ascii_alphabetic() {
            initials.push(c.to_ascii_lowercase());
        } else if let Some(initial) = char_to_initial(c) {
            initials.push(initial);
        }
    }
    initials
}

fn char_to_initial(c: char) -> Option<char> {
    let initial = match c {
        '北' | '百' | '包' | '变' | '部' | '本' | '表' | '备' | '保' | '标' => 'b',
        '产' | '成' | '出' | '长' | '常' | '场' | '测' | '从' | '存' | '超' => 'c',
        '大' | '地' | '定' | '当' | '电' | '点' | '东' | '度' | '道' | '第' => 'd',
        '发' | '分' | '方' | '风' | '复' | '服' | '防' | '非' => 'f',
        '国' | '高' | '公' | '广' | '关' | '各' | '规' | '管' | '工' | '果' => 'g',
        '海' | '和' | '会' | '后' | '行' | '化' | '华' | '环' | '户' => 'h',
        '京' | '机' | '家' | '加' | '建' | '集' | '金' | '经' | '据' | '基' | '监' => 'j',
        '开' | '看' | '可' | '快' | '库' | '控' | '科' => 'k',
        '老' | '量' | '理' | '力' | '两' | '路' | '联' | '流' | '列' | '令' => 'l',
        '美' | '名' | '民' | '明' | '面' | '门' | '目' | '母' | '模' | '每' => 'm',
        '南' | '内' | '年' | '能' | '难' | '农' => 'n',
        '平' | '品' | '片' | '排' | '配' | '盘' => 'p',
        '前' | '全' | '区' | '情' | '强' | '期' | '器' | '群' | '企' | '取' => 'q',
        '人' | '日' | '入' | '如' | '任' | '容' | '热' => 'r',
        '生' | '上' | '实' | '时' | '事' | '手' | '市' | '深' | '数' | '试' | '算' | '收' => 's',
        '天' | '同' | '通' | '头' | '体' | '台' | '特' | '提' | '统' | '图' => 't',
        '万' | '文' | '问' | '无' | '网' | '微' | '物' | '务' | '外' | '位' => 'w',
        '小' | '新' | '下' | '系' | '线' | '信' | '现' | '项' | '型' | '需' => 'x',
        '一' | '有' | '用' | '要' | '以' | '月' | '应' | '原' | '运' | '云' | '营' | '域' => 'y',
        '中' | '主' | '正' | '自' | '在' | '总' | '政' | '重' | '站' | '转' | '州' => 'z',

        _ => return None,
    };
    Some(initial)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_matches() {
        let server = ServerEntry::new("北京生产服务器01", "10.0.1.10", "administrator");

        // 中文名称匹配
        assert!(ServerSearch::matches(&server, "生产"));
        // IP 匹配
        assert!(ServerSearch::matches(&server, "10.0.1"));
        // 用户名匹配
        assert!(ServerSearch::matches(&server, "admin"));
        // 简拼匹配（“北京生产” -> "bjsc"）
        assert!(ServerSearch::matches(&server, "bj"));
        assert!(ServerSearch::matches(&server, "sc"));
        // 未命中
        assert!(!ServerSearch::matches(&server, "shanghai"));
    }
}
