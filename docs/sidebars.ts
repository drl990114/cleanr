import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  tutorialSidebar: [
    {
      type: 'category',
      label: 'Get started',
      collapsed: false,
      items: ['intro', 'quick-start', 'using-cleanr', 'safety-and-recovery'],
    },
    {
      type: 'category',
      label: 'Customize',
      collapsed: false,
      items: ['configuration', 'rules', 'plugins'],
    },
    {
      type: 'category',
      label: 'Help',
      collapsed: false,
      items: ['troubleshooting'],
    },
    {
      type: 'category',
      label: 'Project',
      items: ['architecture', 'development', 'roadmap'],
    },
  ],
};

export default sidebars;
