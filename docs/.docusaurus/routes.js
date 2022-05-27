
import React from 'react';
import ComponentCreator from '@docusaurus/ComponentCreator';

export default [
  {
    path: '/en/blog',
    component: ComponentCreator('/en/blog','aad'),
    exact: true
  },
  {
    path: '/en/blog/archive',
    component: ComponentCreator('/en/blog/archive','679'),
    exact: true
  },
  {
    path: '/en/blog/greetings',
    component: ComponentCreator('/en/blog/greetings','2ae'),
    exact: true
  },
  {
    path: '/en/blog/tags',
    component: ComponentCreator('/en/blog/tags','cc2'),
    exact: true
  },
  {
    path: '/en/blog/tags/greetings',
    component: ComponentCreator('/en/blog/tags/greetings','e15'),
    exact: true
  },
  {
    path: '/en/blog/tags/pisanix',
    component: ComponentCreator('/en/blog/tags/pisanix','d76'),
    exact: true
  },
  {
    path: '/en/docs',
    component: ComponentCreator('/en/docs','3e0'),
    routes: [
      {
        path: '/en/docs/Features/loadbalancer',
        component: ComponentCreator('/en/docs/Features/loadbalancer','ba4'),
        exact: true,
        sidebar: "tutorialSidebar"
      },
      {
        path: '/en/docs/Features/mysql-protocol',
        component: ComponentCreator('/en/docs/Features/mysql-protocol','c85'),
        exact: true,
        sidebar: "tutorialSidebar"
      },
      {
        path: '/en/docs/Features/sql-breaker-limit',
        component: ComponentCreator('/en/docs/Features/sql-breaker-limit','771'),
        exact: true,
        sidebar: "tutorialSidebar"
      },
      {
        path: '/en/docs/Features/sql-parser',
        component: ComponentCreator('/en/docs/Features/sql-parser','28a'),
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
    component: ComponentCreator('/en/','713'),
    exact: true
  },
  {
    path: '*',
    component: ComponentCreator('*')
  }
];
