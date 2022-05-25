
import React from 'react';
import ComponentCreator from '@docusaurus/ComponentCreator';

export default [
  {
    path: '/en/blog',
    component: ComponentCreator('/en/blog','d0a'),
    exact: true
  },
  {
    path: '/en/blog/archive',
    component: ComponentCreator('/en/blog/archive','9c5'),
    exact: true
  },
  {
    path: '/en/blog/greetings',
    component: ComponentCreator('/en/blog/greetings','0b0'),
    exact: true
  },
  {
    path: '/en/blog/tags',
    component: ComponentCreator('/en/blog/tags','6f2'),
    exact: true
  },
  {
    path: '/en/blog/tags/greetings',
    component: ComponentCreator('/en/blog/tags/greetings','d80'),
    exact: true
  },
  {
    path: '/en/blog/tags/pisanix',
    component: ComponentCreator('/en/blog/tags/pisanix','f85'),
    exact: true
  },
  {
    path: '/en/docs',
    component: ComponentCreator('/en/docs','f42'),
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
    component: ComponentCreator('/en/','ceb'),
    exact: true
  },
  {
    path: '*',
    component: ComponentCreator('*')
  }
];
