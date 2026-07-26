import Nav from "../components/landing/Nav";
import Hero from "../components/landing/Hero";
import ProtocolStrip from "../components/landing/ProtocolStrip";
import FeatureGrid from "../components/landing/FeatureGrid";
import MultiProtocol from "../components/landing/MultiProtocol";
import Collections from "../components/landing/Collections";
import MockServer from "../components/landing/MockServer";
import Collaboration from "../components/landing/Collaboration";
import Scripting from "../components/landing/Scripting";
import CodeGen from "../components/landing/CodeGen";
import AiIntegration from "../components/landing/AiIntegration";
import InstallCta from "../components/landing/InstallCta";
import Footer from "../components/landing/Footer";
import ScrollReveal from "../components/landing/ScrollReveal";

export default function LandingPage() {
  return (
    <div className="landing">
      <ScrollReveal />
      <Nav />
      <main>
        <Hero />
        <ProtocolStrip />
        <FeatureGrid />
        <MultiProtocol />
        <Collections />
        <MockServer />
        <Collaboration />
        <Scripting />
        <CodeGen />
        <AiIntegration />
        <InstallCta />
      </main>
      <Footer />
    </div>
  );
}
