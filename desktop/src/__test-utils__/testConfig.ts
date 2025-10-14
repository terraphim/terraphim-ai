export function createTestConfig() {
	return {
		roles: {
			TestRole: {
				name: 'TestRole',
				shortname: 'test',
				relevance_function: 'TitleScorer',
				theme: 'spacelab',
				haystacks: [],
				extra: {},
				terraphim_it: false,
			},
		},
		global_shortcut: 'Ctrl+T',
		theme: 'spacelab',
	};
}
