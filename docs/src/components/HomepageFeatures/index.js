import React from "react";
import clsx from "clsx";
import styles from "./styles.module.css";
import Translate, { translate } from "@docusaurus/Translate";

const FeatureList = [
	{
		title: <Translate id="feat-1" description="sql aware feat">SQL 感知的流量治理</Translate>,
		Svg: require("@site/static/img/undraw_docusaurus_mountain.svg").default,
		description: <Translate id="desc-1" description="sql aware desc">基于 SQL 的负载均衡、审计和可观测性</Translate>,
	},
	{
		title: <Translate id="feat-2" description="runtime mgmt">面向运行时的资源管理</Translate>,
		Svg: require("@site/static/img/undraw_docusaurus_tree.svg").default,
		description: <Translate id="desc-2" description="runtime mgmt desc">实现面向代理运行时的资源管理，如流量 QoS </Translate>,
	},
	{
		title: <Translate id="feat-3" description="dbre">数据库可靠性工程</Translate>,
		Svg: require("@site/static/img/undraw_docusaurus_react.svg").default,
		description: <Translate id="desc-3" description="dbre desc">帮助实现云原生时代的数据库可靠性工程</Translate>,
	},
];

function Feature({ Svg, title, description }) {
	return (
		<div className={clsx("col col--4")}>
			<div className="text--center">
				<Svg className={styles.featureSvg} role="img" />
			</div>
			<div className="text--center padding-horiz--md">
				<h3>{title}</h3>
				<p>{description}</p>
			</div>
	</div>
	);
}

export default function HomepageFeatures() {
	return (
		<section className={styles.features}>
			<div className="container">
				<div className="row">
					{FeatureList.map((props, idx) => (
						<Feature key={idx} {...props} />
					))}
				</div>
			</div>
		</section>
	);
}
