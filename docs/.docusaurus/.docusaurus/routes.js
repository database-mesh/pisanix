
import React from 'react';
import ComponentCreator from '@docusaurus/ComponentCreator';

export default [
  {
    path: '/en/blog',
    component: ComponentCreator('/en/blog','586'),
    exact: true
  },
  {
    path: '/en/blog/archive',
    component: ComponentCreator('/en/blog/archive','322'),
    exact: true
  },
  {
    path: '/en/blog/greetings',
    component: ComponentCreator('/en/blog/greetings','6bd'),
    exact: true
  },
  {
    path: '/en/blog/tags',
    component: ComponentCreator('/en/blog/tags','e83'),
    exact: true
  },
  {
    path: '/en/blog/tags/greetings',
    component: ComponentCreator('/en/blog/tags/greetings','4a9'),
    exact: true
  },
  {
    path: '/en/blog/tags/pisanix',
    component: ComponentCreator('/en/blog/tags/pisanix','7cb'),
    exact: true
  },
  {
    path: '/en/markdown-page',
    component: ComponentCreator('/en/markdown-page','dd2'),
    exact: true
  },
  {
    path: '/en/docs',
    component: ComponentCreator('/en/docs','233'),
    routes: [
      {
        path: '/en/docs/Features/database-reliability-engineering',
        component: ComponentCreator('/en/docs/Features/database-reliability-engineering','6d4'),
        exact: true,
        sidebar: "tutorialSidebar"
      },
      {
        path: '/en/docs/Features/runtime-oriented-resource-mgmt',
        component: ComponentCreator('/en/docs/Features/runtime-oriented-resource-mgmt','9c7'),
        exact: true,
        sidebar: "tutorialSidebar"
      },
      {
        path: '/en/docs/Features/sql-aware-traffic-governance',
        component: ComponentCreator('/en/docs/Features/sql-aware-traffic-governance','b02'),
        exact: true,
        sidebar: "tutorialSidebar"
      },
      {
        path: '/en/docs/intro',
        component: ComponentCreator('/en/docs/intro','7c8'),
        exact: true,
        sidebar: "tutorialSidebar"
      },
      {
        path: '/en/docs/quickstart',
        component: ComponentCreator('/en/docs/quickstart','2be'),
        exact: true,
        sidebar: "tutorialSidebar"
      },
      {
        path: '/en/docs/UseCases/kubernetes',
        component: ComponentCreator('/en/docs/UseCases/kubernetes','402'),
        exact: true,
        sidebar: "tutorialSidebar"
      },
      {
        path: '/en/docs/UseCases/standalone',
        component: ComponentCreator('/en/docs/UseCases/standalone','bb8'),
        exact: true,
        sidebar: "tutorialSidebar"
      }
    ]
  },
  {
    path: '/en/',
    component: ComponentCreator('/en/','c08'),
    exact: true
  },
  {
    path: '*',
    component: ComponentCreator('*')
  }
];
